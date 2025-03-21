// use std::io;
mod module;
pub use module::Module;

mod app;
pub use app::App;

mod signal;
pub use signal::Signal;

use std::io;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default()?.run(&mut terminal);
    ratatui::restore();
    app_result
}
