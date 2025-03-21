use std::{
    fs::File,
    io::{self, BufReader},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use vcd::{ScopeItem, Value};

use crate::{Module, Signal};

use crate::signal::ValueType;

#[derive(PartialEq)]
enum AppMode {
    Run,
    Exit,
}

pub struct App {
    module_root: Module,

    mode: AppMode,
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

    Ok(root)
}

impl App {
    pub fn default() -> io::Result<Self> {
        let module_root = parse_files(String::from("./src/test_1.vcd"))?;
        Ok(Self {
            mode: AppMode::Run,
            module_root,
        })
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn draw(&self, frame: &mut ratatui::Frame<'_>) {
        let layouts = Layout::default()
            .direction(Direction::Horizontal)
            .margin(2)
            .constraints([Constraint::Fill(3), Constraint::Fill(7)].as_ref())
            .split(frame.area());

        let signals = Paragraph::new(format!("{}", self.module_root))
            .block(Block::default().borders(Borders::ALL).title("Signals"));
        frame.render_widget(signals, layouts[0]);

        // let waveform = Block::new().borders(Borders::ALL).title("Waveform");
        // const Low_line: &str = "_";
        // const Combining_Low_line: &str = "\u{0332}";
        // const FULLWIDTH_Low_line: &str = "\u{FF3F}";
        // const High_line: &str = "\u{0305}";
        // const Combining_overline: &str = "\u{203E}";
        // const Vertical: &str = "\u{20D2}";
        // const Combining_long_vertical_line_overlay: &str = "\u{20D2}";
        // const Vertical_line: &str = "\u{007C}";
        // const Modifier_letter_extra_low_tone_bar: &str = "\u{02E9}";
        // let symbols = vec![
        //     Low_line,
        //     Combining_Low_line,
        //     FULLWIDTH_Low_line,
        //     High_line,
        //     Combining_overline,
        //     Vertical,
        //     Combining_long_vertical_line_overlay,
        //     Vertical_line,
        //     Modifier_letter_extra_low_tone_bar,
        // ];
        // let RISING_EDGE = format!("{}{}", Low_line, Combining_long_vertical_line_overlay);
        // let FALLING_EDGE = format!("{}{}", High_line, Combining_long_vertical_line_overlay);
        let wave_line = Line::from(
            "__________\u{20D2}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}",
        );
        let waveform = Paragraph::new(wave_line)
            .block(Block::default().borders(Borders::ALL).title("Waveform"));

        frame.render_widget(waveform, layouts[1]);
    }

    fn handle_key_event(&mut self, key_event: event::KeyEvent) {
        match self.mode {
            AppMode::Run => match key_event.code {
                KeyCode::Char('q') => {
                    self.mode = AppMode::Exit;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.mode != AppMode::Exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }
}
