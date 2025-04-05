use cli_log::*;
use rata_wave::app::App;

use std::io;

fn main() -> io::Result<()> {
    init_cli_log!();
    let mut terminal = ratatui::init();
    let app_result = App::default()?.run(&mut terminal);
    ratatui::restore();
    app_result
}
