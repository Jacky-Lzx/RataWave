// use std::io;

// use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
// use ratatui::layout::{Constraint, Direction, Layout};
// use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
// use ratatui::{DefaultTerminal, Frame};

/*
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}
*/

/*
pub struct App {}

impl App {
    pub fn new() -> Self {
        todo!();
        Self {}
    }
    pub fn default() -> Self {
        todo!();
        Self {}
    }
}

impl App {
    pub fn run(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        todo!();
    }
}
*/

use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::ErrorKind::InvalidInput;

/// Parse a VCD file containing a clocked signal and decode the signal
fn read_clocked_vcd(r: &mut dyn io::BufRead) -> io::Result<()> {
    let mut parser = vcd::Parser::new(r);

    // Parse the header and find the wires
    let header = parser.parse_header()?;

    let clock_code = header
        .find_var(&["test_tb", "clk"])
        .ok_or_else(|| io::Error::new(InvalidInput, "no wire test_tb.clk"))?
        .code;

    let mut clock_vec = vec![];

    for command_result in parser {
        let command = command_result?;
        use vcd::Command::*;
        match command {
            ChangeScalar(i, v) if i == clock_code => {
                clock_vec.push(v.to_string());
            }
            _ => (),
        }
    }

    let result = clock_vec.join(" ");

    println!("{result}");

    Ok(())
}

fn main() -> std::io::Result<()> {
    let file_path = "./src/test.vcd";

    println!("In file {file_path}");

    let f = File::open(file_path)?;
    let mut f = BufReader::new(f);

    // let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    let value = read_clocked_vcd(&mut f);

    value
}
