use std::{
    fs::File,
    io::{self, BufReader},
};

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::Style,
    text::Span,
};
use vcd::{ScopeItem, TimescaleUnit, Value, Vector};

use crate::{
    module::Module,
    signal::{Signal, ValueType},
};

pub fn parse_files(file_name: String) -> io::Result<(Module, TimescaleUnit)> {
    let mut root = Module {
        name: String::from("Root"),
        depth: 1,
        signals: vec![],
        submodules: vec![],
    };

    let mut parser = vcd::Parser::new(BufReader::new(File::open(file_name)?));

    // Parse the header and find the wires
    let header = parser.parse_header()?;

    assert!(header.timescale.unwrap().0 == 1);

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

    let mut cur_time_stamp = 0;
    for command_result in parser {
        let command = command_result?;
        use vcd::Command::*;
        match command {
            Timestamp(t) => {
                cur_time_stamp = t;
            }
            ChangeScalar(id, value) => {
                root.add_event(id, cur_time_stamp, ValueType::Value(value));
            }
            ChangeVector(id, vector) => {
                root.add_event(id, cur_time_stamp, ValueType::Vector(vector));
            }
            _ => (),
        }
    }

    Ok((root, header.timescale.unwrap().1))
}

pub fn middle_str<'a>(length: usize, mid_str: String) -> Vec<Span<'a>> {
    let len = mid_str.len();
    if len > length {
        return vec![Span::styled("␩", Style::default()); length];
    }
    let mut arr = vec![];
    for _ in 0..length {
        arr.push(Span::styled(" ", Style::default()));
    }
    // ␩
    arr.splice(
        length / 2 - len / 2..length / 2 - len / 2 + len,
        mid_str
            .chars()
            .map(|x| Span::styled(x.to_string(), Style::default())),
    );

    assert!(arr.len() == length);

    arr
}

pub fn vector_contain_x_or_z(vector: &Vector) -> bool {
    vector
        .iter()
        .find(|&x| x == Value::X || x == Value::Z)
        .iter()
        .count()
        != 0
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
