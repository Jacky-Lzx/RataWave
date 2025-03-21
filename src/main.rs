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

use core::fmt;
use std::io;
use std::io::BufReader;
use std::io::ErrorKind::InvalidInput;
use std::{fmt::Display, fs::File};

use vcd::ScopeItem;

struct MyScopeItem<'a>(&'a ScopeItem, i8);

impl<'a> Display for MyScopeItem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ScopeItem::*;
        match &self.0 {
            Comment(comment) => {
                writeln!(f, "Comment: {comment}")?;
            }
            Var(var) => {
                let var_name = &var.reference;
                writeln!(f, "Var: {var_name}")?;
            }
            Scope(scope) => {
                let scope_name = &scope.identifier;
                writeln!(f, "Scope: {scope_name}")?;
                scope.items.iter().try_for_each(|x| -> fmt::Result {
                    let my_scope_item = MyScopeItem(x, self.1 + 1);
                    for _ in 1..self.1 {
                        write!(f, "  ")?;
                    }
                    writeln!(f, "{my_scope_item}")?;
                    Ok(())
                })?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Parse a VCD file containing a clocked signal and decode the signal
fn read_clocked_vcd(r: &mut dyn io::BufRead) -> io::Result<()> {
    let mut parser = vcd::Parser::new(r);

    // Parse the header and find the wires
    let header = parser.parse_header()?;

    header.items.iter().for_each(|x| {
        let my_scope_item = MyScopeItem(x, 1);
        println!("{my_scope_item}");
    });

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
    let file_path = "./src/test_1.vcd";

    println!("In file {file_path}");

    let f = File::open(file_path)?;
    let mut f = BufReader::new(f);

    // let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    let value = read_clocked_vcd(&mut f);

    value
}
