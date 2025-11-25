use clap::Parser;
use cli_log::*;
use rata_wave::{app::App, modules::cli_args::CliArgs};

use std::io;

fn main() -> io::Result<()> {
    let args = CliArgs::parse();

    init_cli_log!();
    let mut terminal = ratatui::init();
    let app_result = App::default(args)?.run(&mut terminal);
    ratatui::restore();
    app_result
}
