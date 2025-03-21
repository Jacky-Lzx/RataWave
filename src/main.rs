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

use vcd::{Header, IdCode, Scope, ScopeItem, ScopeType, Var};

struct MyScopeItem<'a>(&'a ScopeItem, i8);

struct Signal {
    // reference string in vcd file
    code: IdCode,
    name: String,
    events: Vec<(u64, u64)>,
}

impl Signal {
    fn from_var(var: &Var) -> Signal {
        Signal {
            code: var.code,
            name: var.reference.clone(),
            events: vec![],
        }
    }
}

impl Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.events.len() == 0 {
            writeln!(f, "Signal: {}, code: {}", self.name, self.code)?;
        } else {
            writeln!(
                f,
                "Signal: {}, code: {}, events: {:?}",
                self.name, self.code, self.events
            )?;
        }
        Ok(())
    }
}

struct Module {
    name: String,
    depth: u8,
    signals: Vec<Signal>,
    submodules: Vec<Module>,
}

impl Module {
    fn from_scope(scope: &Scope, depth: u8) -> Module {
        assert!(scope.scope_type == ScopeType::Module);
        let mut signals = vec![];
        let mut sub_modules = vec![];

        for scope_type in &scope.items {
            match scope_type {
                ScopeItem::Var(var) => {
                    signals.push(Signal::from_var(var));
                }
                ScopeItem::Scope(sub_scope) => {
                    sub_modules.push(Module::from_scope(sub_scope, depth + 1))
                }
                _ => {}
            }
        }

        Module {
            name: scope.identifier.clone(),
            depth,
            signals,
            submodules: sub_modules,
        }
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Module: {}, depth: {}", self.name, self.depth)?;
        self.signals.iter().try_for_each(|x| {
            for _ in 0..self.depth {
                write!(f, "  ")?;
            }
            write!(f, "{x}")?;
            Ok(())
        })?;

        self.submodules.iter().try_for_each(|x| {
            for _ in 0..self.depth {
                write!(f, "  ")?;
            }
            write!(f, "{x}")?;
            Ok(())
        })?;
        Ok(())
    }
}

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
                    for _ in 0..self.1 {
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

fn parse_modules(header: &Header) -> io::Result<Module> {
    let mut root = Module {
        name: String::from("Root"),
        depth: 1,
        signals: vec![],
        submodules: vec![],
    };

    header.items.iter().for_each(|x| {
        // let my_scope_item = MyScopeItem(x, 1);
        // println!("{my_scope_item}");
        use ScopeItem::*;
        match x {
            Scope(scope) => {
                root.submodules
                    .push(Module::from_scope(scope, root.depth + 1));
            }
            Var(var) => {
                root.signals.push(Signal::from_var(var));
            }
            _ => {}
        }
    });
    // root.submodules = root.submodules.as_mut().map(|x| )

    Ok(root)
}

/// Parse a VCD file containing a clocked signal and decode the signal
fn read_clocked_vcd(r: &mut dyn io::BufRead) -> io::Result<()> {
    let mut parser = vcd::Parser::new(r);

    // Parse the header and find the wires
    let header = parser.parse_header()?;

    // header.items.iter().for_each(|x| {
    //     let my_scope_item = MyScopeItem(x, 1);
    //     println!("{my_scope_item}");
    // });

    let root = parse_modules(&header)?;

    println!("{root}");

    todo!();

    let clock_code = header
        .find_var(&["test_tb", "clk"])
        .ok_or_else(|| io::Error::new(InvalidInput, "no wire test_tb.clk"))?
        .code;

    let rst_code = header
        .find_var(&["test_tb", "rst"])
        .ok_or_else(|| io::Error::new(InvalidInput, "no wire test_tb.rst"))?
        .code;

    let mut clock_vec = vec![];
    let mut rst_vec = vec![];

    let mut cur_time_stamp = 0;
    for command_result in parser {
        let command = command_result?;
        use vcd::Command::*;
        match command {
            Timestamp(t) => {
                cur_time_stamp = t;
            }
            ChangeScalar(i, v) if i == clock_code => {
                clock_vec.push((cur_time_stamp, v.to_string()));
            }
            ChangeScalar(i, v) if i == rst_code => {
                rst_vec.push((cur_time_stamp, v.to_string()));
            }
            _ => (),
        }
    }

    print!("clk: ");
    clock_vec.iter().for_each(|x| println!("{x:?}"));

    print!("rst: ");
    rst_vec.iter().for_each(|x| println!("{x:?}"));

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
