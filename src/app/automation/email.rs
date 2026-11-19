use lettre::{
    Message, SmtpTransport, Transport,
    message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use std::{fs, path::PathBuf, time::Duration};

pub(super) struct MailDelivery<'a> {
    pub host: &'a str,
    pub port: Option<u16>,
    pub starttls: bool,
    pub username: Option<&'a str>,
    pub password_env: Option<&'a str>,
}

pub(super) fn valid_mailbox(value: &str) -> bool {
    !value.contains(['\r', '\n']) && value.parse::<Mailbox>().is_ok()
}

fn message(
    from: &str,
    to: &[String],
    subject: &str,
    body: &str,
    attachments: &[PathBuf],
) -> Result<Message, String> {
    if subject.trim().is_empty() || subject.contains(['\r', '\n']) {
        return Err("email subject must be a nonempty single line".into());
    }
    let mut headers = Message::builder()
        .from(from.parse::<Mailbox>().map_err(|_| "invalid SMTP sender")?)
        .subject(subject);
    for recipient in to {
        headers = headers.to(recipient
            .parse::<Mailbox>()
            .map_err(|_| "invalid SMTP recipient")?);
    }
    if attachments.is_empty() {
        return headers
            .singlepart(SinglePart::plain(body.to_owned()))
            .map_err(|_| "unable to build email".into());
    }
    let mut parts = MultiPart::mixed().singlepart(SinglePart::plain(body.to_owned()));
    for path in attachments {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.contains(['\r', '\n']))
            .ok_or("attachment needs a UTF-8 filename")?;
        if !path.is_file() {
            return Err(format!(
                "attachment must be a regular file: {}",
                path.display()
            ));
        }
        let contents = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        parts = parts.singlepart(
            Attachment::new(filename.into()).body(
                contents,
                ContentType::parse("application/octet-stream")
                    .map_err(|_| "invalid attachment content type")?,
            ),
        );
    }
    headers
        .multipart(parts)
        .map_err(|_| "unable to build email attachments".into())
}

pub(super) fn deliver(
    relay: &MailDelivery<'_>,
    from: &str,
    to: &[String],
    subject: &str,
    body: &str,
    attachments: &[PathBuf],
) -> Result<(), String> {
    let message = message(from, to, subject, body, attachments)?;
    let mut client = if relay.starttls {
        SmtpTransport::starttls_relay(relay.host)
    } else {
        SmtpTransport::relay(relay.host)
    }
    .map_err(|_| "invalid SMTP TLS relay hostname")?
    .timeout(Some(Duration::from_secs(30)));
    if let Some(port) = relay.port {
        client = client.port(port);
    }
    if let (Some(username), Some(variable)) = (relay.username, relay.password_env) {
        let password = std::env::var(variable)
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("SMTP password environment variable {variable} is unset or empty")
            })?;
        client = client.credentials(Credentials::new(username.into(), password));
    }
    client.build().send(&message).map(|_| ()).map_err(|error| {
        // The relay controls its response text, so include only the diagnostic category.
        if error.is_timeout() {
            "SMTP delivery timed out"
        } else if error.is_transient() {
            "SMTP relay returned a temporary failure"
        } else if error.is_permanent() {
            "SMTP relay rejected delivery"
        } else {
            "SMTP connection or delivery failed"
        }
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
    };

    #[test]
    fn message_formats_mime_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let attachment = dir.path().join("report.txt");
        fs::write(&attachment, "Example report").unwrap();
        let mail = message(
            "sender@example.org",
            &["recipient@example.org".into()],
            "Run result",
            "Example body",
            &[attachment],
        )
        .unwrap();
        let envelope = mail.envelope();
        assert_eq!(envelope.to()[0].to_string(), "recipient@example.org");
        let mime = String::from_utf8(mail.formatted()).unwrap();
        assert!(mime.contains("Subject: Run result"));
        assert!(mime.contains("Example body"));
        assert!(mime.contains("report.txt"));
        assert!(
            message(
                "sender@example.org",
                &["recipient@example.org".into()],
                "bad\nBcc: other@example.org",
                "",
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn starttls_refuses_plaintext_delivery_to_local_fake_relay() {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let relay = std::thread::spawn(move || {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream.write_all(b"220 fake.relay ESMTP\r\n").unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with("EHLO "));
            reader.get_mut().write_all(b"250 fake.relay\r\n").unwrap();
            line.clear();
            let _ = reader.read_line(&mut line);
            line
        });
        let result = deliver(
            &MailDelivery {
                host: "127.0.0.1",
                port: Some(port),
                starttls: true,
                username: None,
                password_env: None,
            },
            "sender@example.org",
            &["recipient@example.org".into()],
            "Run result",
            "Private report",
            &[],
        );
        assert!(result.is_err());
        let command = relay.join().unwrap();
        assert!(command.is_empty() || command.starts_with("QUIT"));
    }
}
