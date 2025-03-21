use std::{
    fs::File,
    io::{self, BufReader},
};

use ratatui::DefaultTerminal;
use vcd::ScopeItem;

use crate::{Module, Signal};

pub struct App {
    module_root: Module,
}

fn parse_files(file_name: String) -> io::Result<Module> {
    let mut root = Module {
        name: String::from("Root"),
        depth: 1,
        signals: vec![],
        submodules: vec![],
    };

    let mut parser = vcd::Parser::new(BufReader::new(File::open(file_name)?));

    // Parse the header and find the wires
    let header = parser.parse_header()?;

    header.items.iter().for_each(|x| {
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

    Ok(root)
}

impl App {
    pub fn default() -> io::Result<Self> {
        let module_root = parse_files(String::from("./src/test_1.vcd"))?;
        Ok(Self { module_root })
    }
}

impl App {
    pub fn run(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        todo!();
        println!("{}", self.module_root);
        Ok(())
    }
}
