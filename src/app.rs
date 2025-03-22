use std::{
    cmp::max,
    fs::File,
    io::{self, BufReader},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{self, Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph, Sparkline},
};
use vcd::{ScopeItem, Value};

use crate::{Module, Signal, signal::vector_to_base_10};

use crate::signal::ValueType;

#[derive(PartialEq)]
enum AppMode {
    Run,
    Exit,
}

pub struct App {
    module_root: Module,
    time_max: u64,
    time_split: u64,

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
        let time_max = module_root.max_time();
        Ok(Self {
            mode: AppMode::Run,
            module_root,
            time_max,
            time_split: 1,
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
        .block(Block::default().borders(Borders::ALL).title("Signals"));

        // let signals = Paragraph::new(format!("{}", self.module_root))
        //     .block(Block::default().borders(Borders::ALL).title("Signals"));
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
        // let wave_line = Line::from(
        //     "__________\u{20D2}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{0305}\u{20D2}",
        // );
        // let waveform = Paragraph::new(wave_line)
        //     .block(Block::default().borders(Borders::ALL).title("Waveform"));

        let events =
            Layout::vertical(vec![Constraint::Fill(1); signal_vec.len() * 2]).split(layouts[1]);

        let base: u64 = 2;

        for (index, signal) in signal_vec.iter().enumerate() {
            let single_event: Vec<Option<u64>> = signal
                .event_arr_in_range(
                    (
                        0,
                        self.time_max / base.pow(self.time_split.try_into().unwrap()),
                    ),
                    self.time_max / base.pow(self.time_split.try_into().unwrap()) / 100,
                )
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
                    .join(", "),
            );

            // let test_data = vec![1, 2, 3, 4];

            // let sparkline = Sparkline::default()
            //     .block(
            //         Block::default()
            //             .borders(Borders::ALL)
            //             .title(signal.output_name()),
            //     )
            //     .absent_value_style(Style::default().fg(Color::Red))
            //     .data(&single_event);
            // .data(&test_data);

            frame.render_widget(par, events[index * 2]);
            // frame.render_widget(sparkline, events[index * 2 + 1]);
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
                    self.time_split += 1;
                }
                KeyCode::Char('-') => {
                    // self.time_split -= 1;
                    self.time_split = max(self.time_split - 1, 1)
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
