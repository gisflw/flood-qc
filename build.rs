fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=assets/branding/hge.ico");
    #[cfg(feature = "desktop")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/branding/hge.ico")
            .set("ProductName", "MGB Ops")
            .set("FileDescription", "MGB Ops desktop")
            .compile()?;
    }
    Ok(())
}
