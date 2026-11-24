mod desktop;

fn main() -> eframe::Result {
    if let Some(result) = mgb_ops::app::assist_mcp_entry() {
        return result.map_err(|error| eframe::Error::AppCreation(Box::new(error)));
    }
    desktop::run()
}
