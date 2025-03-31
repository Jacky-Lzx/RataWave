use std::{
    cmp::{max, min},
    fs::File,
    io::{self, BufReader},
};

use cli_log::debug;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use vcd::{ScopeItem, Value};

use crate::{
    Module, Signal,
    signal::{FALLING_EDGE, RISING_EDGE, vector_to_base_10},
};

use crate::signal::ValueType;

#[derive(PartialEq)]
enum AppMode {
    Run,
    Exit,
}

pub struct App {
    module_root: Module,
    time_start: u64,
    time_step: u64,
    arr_size: usize,

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
        debug!("{}", module_root);
        Ok(Self {
            mode: AppMode::Run,
            module_root,
            time_start: 0,
            time_step: 1000,
            arr_size: 100,
        })
    }

    fn change_time_step(&mut self, is_increase: bool) {
        if is_increase {
            self.time_step = max(self.time_step / 10, 10);
        } else {
            self.time_step *= 10;
        }
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

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let layouts = Layout::default()
            .direction(Direction::Horizontal)
            .margin(2)
            .constraints([Constraint::Fill(1), Constraint::Fill(9)].as_ref())
            .split(frame.area());

        let signal_vec = self.module_root.get_signals();

        let signals = Paragraph::new(
            signal_vec
                .iter()
                .map(|x| x.output_name())
                .collect::<Vec<String>>()
                .join("\n"),
        )
        .block(Block::default().borders(Borders::ALL).title("Names"));

        let name_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)].as_ref())
            .split(layouts[0]);

        let redundant = Paragraph::new("RataWave");

        frame.render_widget(redundant, name_layouts[0]);
        frame.render_widget(signals, name_layouts[1]);

        let signal_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)].as_ref())
            .split(layouts[1]);

        let events = Layout::vertical(vec![Constraint::Fill(1); signal_vec.len() * 2])
            .split(signal_layouts[1]);

        self.arr_size = events[0].width as usize;

        let mut time_stamp_str = String::from("");

        // Show stamps after each 10 steps
        let show_split = 10;

        let mut time_stamp_graph = String::from("");

        let mut stamp_index = 0;
        while stamp_index < self.arr_size {
            let mut time_stamp = format!(
                "{}ns",
                self.time_start + stamp_index as u64 * self.time_step
            );
            let strip_len = min(10, self.arr_size - stamp_index);
            if time_stamp.len() > strip_len {
                time_stamp = time_stamp[0..strip_len].to_string();
            } else {
                time_stamp.push_str(" ".repeat(strip_len - time_stamp.len()).as_str());
            }
            time_stamp_str.push_str(&time_stamp);

            time_stamp_graph.push_str(format!("|{}", " ".repeat(strip_len - 1)).as_str());

            stamp_index += show_split;
        }

        let time_show = Paragraph::new(vec![
            Line::from(time_stamp_str),
            Line::from(time_stamp_graph),
        ]);
        // .block(Block::default().borders(Borders::ALL));

        frame.render_widget(time_show, signal_layouts[0]);

        for (index, signal) in signal_vec.iter().enumerate() {
            let single_event: Vec<Option<u64>> = signal
                .events_arr_in_range(self.time_start, self.time_step, self.arr_size)
                .iter()
                .map(|x| match x {
                    ValueType::Value(value) => match value {
                        Value::V0 => Some(0),
                        Value::V1 => Some(1),
                        _ => None,
                    },
                    ValueType::Vector(vector) => vector_to_base_10(vector),
                })
                .collect::<Vec<Option<u64>>>();

            let par = Paragraph::new(
                single_event
                    .iter()
                    .map(|x| match x {
                        Some(n) => format!("{}", n),
                        _ => "x".to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(""),
            );

            // let test_data = vec![1, 2, 3, 4];

            let sparkline = Paragraph::new(self.get_lines_from_a_signal(signal));

            frame.render_widget(par, events[index * 2]);
            frame.render_widget(sparkline, events[index * 2 + 1]);
        }

        // let waveform = Paragraph::new(
        //     signal_vec
        //         .iter()
        //         .map(|x| {
        //             let event_arr = x
        //                 .event_arr_in_range(
        //                     (
        //                         0,
        //                         self.time_max / base.pow(self.time_split.try_into().unwrap()),
        //                     ),
        //                     self.time_max / base.pow(self.time_split.try_into().unwrap()) / 100,
        //                 )
        //                 .iter()
        //                 .map(|x| vector_to_base_10(x))
        //                 .collect();
        //         })
        //         .collect::<Vec<String>>()
        //         .join("\n"),
        // )
        // .block(Block::default().borders(Borders::ALL).title("Waveform"));

        // frame.render_widget(waveform, layouts[1]);
    }

    fn handle_key_event(&mut self, key_event: event::KeyEvent) {
        match self.mode {
            AppMode::Run => match key_event.code {
                KeyCode::Char('q') => {
                    self.mode = AppMode::Exit;
                }
                KeyCode::Char('=') => {
                    self.change_time_step(true);
                }
                KeyCode::Char('-') => {
                    self.change_time_step(false);
                }
                KeyCode::Char('h') => {
                    self.time_start = max(
                        0,
                        self.time_start as i64 - self.arr_size as i64 / 2 * self.time_step as i64,
                    ) as u64;
                }
                KeyCode::Char('l') => {
                    self.time_start += self.arr_size as u64 / 2 * self.time_step;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn get_lines_from_a_signal(&self, signal: &Signal) -> Vec<Line> {
        let lines = signal
            .events_arr_in_range(self.time_start, self.time_step, self.arr_size)
            .windows(2)
            .fold(("".to_string(), "".to_string()), |lines, window| {
                let first = window[0];
                let second = window[1];

                if let ValueType::Vector(_) = first {
                    return ("multiple bits".to_string(), "multiple bits".to_string());
                };

                assert!(matches!(first, ValueType::Value(_)));
                assert!(matches!(second, ValueType::Value(_)));

                let ValueType::Value(first) = first else {
                    todo!()
                };
                let ValueType::Value(second) = second else {
                    todo!()
                };
                if first == second {
                    match first {
                        Value::V1 => {
                            return (lines.0.to_string() + "─", lines.1.to_string() + " ");
                        }
                        Value::V0 => {
                            return (lines.0.to_string() + " ", lines.1.to_string() + "─");
                        }
                        Value::X => return (lines.0.to_string() + "x", lines.1.to_string() + "x"),
                        Value::Z => return (lines.0.to_string() + "z", lines.1.to_string() + "z"),
                    }
                } else {
                    match second {
                        Value::V1 => {
                            return (
                                lines.0 + RISING_EDGE.first_line,
                                lines.1 + RISING_EDGE.second_line,
                            );
                        }
                        Value::V0 => {
                            return (
                                lines.0 + FALLING_EDGE.first_line,
                                lines.1 + FALLING_EDGE.second_line,
                            );
                        }
                        Value::X => return (lines.0.to_string() + "x", lines.1.to_string() + "x"),
                        Value::Z => return (lines.0.to_string() + "z", lines.1.to_string() + "z"),
                    }
                }
            });

        vec![Line::from(lines.0), Line::from(lines.1)]
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
