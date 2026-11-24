use super::{Bridge, Control, Gateway, Shared};
use crate::app::{InspectionRequest, Session};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, watch},
};

fn gateway(root: &Path) -> (Gateway, watch::Sender<Control>) {
    let session = Session::new(InspectionRequest {
        workspace: root.join("workspace"),
        invocation_dir: root.into(),
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    let (control, receiver) = watch::channel(Control::default());
    let (events, _receiver) = mpsc::channel(4);
    let shared = Arc::new(Shared::new());
    shared.begin(1);
    (
        Gateway {
            shared,
            session,
            events,
            control: receiver,
            connected: Arc::new(AtomicBool::default()),
        },
        control,
    )
}

#[test]
fn closing_rejects_the_previous_credential_and_terminates_its_connection() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(3), async {
                let old = Bridge::bind().await.unwrap();
                let replacement = Bridge::bind().await.unwrap();
                let old_address = old.listener.local_addr().unwrap();
                let new_address = replacement.listener.local_addr().unwrap();
                let old_token = old.token;
                let new_token = replacement.token;
                let (old_gateway, old_control) = gateway(Path::new("old"));
                let (replacement_gateway, _replacement_control) = gateway(Path::new("new"));
                let state = old_gateway.shared.clone();
                tokio::spawn(old.serve(old_gateway));
                tokio::spawn(replacement.serve(replacement_gateway));

                let mut connected = TcpStream::connect(old_address).await.unwrap();
                connected.write_all(&old_token).await.unwrap();
                assert_eq!(connected.read_u8().await.unwrap(), 1);
                state.close();
                old_control.send_modify(|state| state.closed = true);
                let mut byte = [0];
                assert_eq!(connected.read(&mut byte).await.unwrap(), 0);

                let mut old_reconnect = TcpStream::connect(old_address).await.unwrap();
                let _ = old_reconnect.write_all(&old_token).await;
                let closed_or_rejected = old_reconnect.read_u8().await.is_err();
                assert!(closed_or_rejected);
                let mut cross_session = TcpStream::connect(new_address).await.unwrap();
                cross_session.write_all(&old_token).await.unwrap();
                assert_eq!(cross_session.read(&mut byte).await.unwrap(), 0);
                let mut connected = TcpStream::connect(new_address).await.unwrap();
                connected.write_all(&new_token).await.unwrap();
                assert_eq!(connected.read_u8().await.unwrap(), 1);
            })
            .await
            .unwrap();
        });
}
