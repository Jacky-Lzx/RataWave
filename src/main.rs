use cli_log::*;

use std::io;
mod module;
pub use module::Module;
mod app;
pub use app::App;

mod signal;
pub use signal::Signal;

fn main() -> io::Result<()> {
    init_cli_log!();
    let mut terminal = ratatui::init();
    let app_result = App::default()?.run(&mut terminal);
    ratatui::restore();
    app_result
}
