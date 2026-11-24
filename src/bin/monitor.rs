mod desktop;

fn main() -> eframe::Result {
    let mut arguments = std::env::args_os().skip(1);
    if arguments
        .next()
        .is_some_and(|arg| arg == "--assistant-mcp-stdio")
        && arguments.next().is_none()
    {
        if let Err(error) = mgb_ops::app::assistant_mcp_stdio() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    desktop::run()
}
