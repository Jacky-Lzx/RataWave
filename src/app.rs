use std::{
    cmp::{max, min},
    fmt::Display,
    fs::File,
    io::{self, BufReader},
    ops::Add,
};

use cli_log::{debug, error};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use vcd::{ScopeItem, TimescaleUnit, Value};

use crate::{
    Module, Signal,
    signal::{self, FALLING_EDGE, RISING_EDGE, vector_to_base_10},
};

use crate::signal::ValueType;

#[derive(PartialEq)]
enum AppMode {
    Run,
    Exit,
}

#[derive(Clone)]
struct Time {
    // Stored in ps
    time: u64,
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut t: f64 = self.time as f64;
        // let mut scale = TimescaleUnit::PS;
        use TimescaleUnit::*;
        let scales = [PS, NS, US, MS, S];
        let scale = scales
            .iter()
            .rfind(|x| t >= (PS.divisor() / x.divisor()) as f64)
            .unwrap_or(&PS);
        t = t / (PS.divisor() / scale.divisor()) as f64;
        write!(f, "{}{}", t, scale)
    }
}

impl Add<u64> for Time {
    type Output = Time;

    fn add(self, rhs: u64) -> Self::Output {
        Time {
            time: self.time + rhs,
        }
    }
}

impl Time {
    pub fn new(time: u64, unit: TimescaleUnit) -> Self {
        let time_in_ps = time * TimescaleUnit::PS.divisor() / unit.divisor();
        Time { time: time_in_ps }
    }

    pub fn increase(&mut self, time_inc: u64) {
        self.time += time_inc;
    }

    pub fn decrease(&mut self, time_dec: u64) {
        self.time = if self.time < time_dec {
            0
        } else {
            self.time - time_dec
        }
    }

    pub fn time(&self) -> u64 {
        self.time
    }

    pub fn formulate(&self) -> u64 {
        let mut t = self.time;
        while t >= 1000 {
            if t % 1000 != 0 {
                panic!("self.time can not divides 1000!")
            }
            t /= 1000;
        }
        t
    }

    pub fn step_decrease(&mut self) {
        self.time = match self.formulate() {
            1 | 10 | 100 => max(1, self.time / 2),
            5 | 50 | 500 => self.time / 5,
            _ => panic!("Invalid time step: {}", self.time),
        }
    }
    pub fn step_increase(&mut self) {
        self.time = match self.formulate() {
            1 | 10 | 100 => self.time * 5,
            5 | 50 | 500 => self.time * 2,
            _ => panic!("Invalid time step: {}", self.time),
        }
    }
}

pub struct App {
    module_root: Module,
    time_start: Time,
    time_step: Time,
    arr_size: usize,
    // time_scale: TimescaleUnit,
    mode: AppMode,
}

fn parse_files(file_name: String) -> io::Result<(Module, TimescaleUnit)> {
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

impl App {
    pub fn default() -> io::Result<Self> {
        let (module_root, time_base_scale) = parse_files(String::from("./src/test_1.vcd"))?;
        Ok(Self {
            mode: AppMode::Run,
            module_root,
            time_start: Time::new(0, time_base_scale),
            time_step: Time::new(10, time_base_scale),
            arr_size: 100,
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

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let layouts = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(frame.area());

        let signal_vec = self.module_root.get_signals();

        // let signals = Paragraph::new(
        //     signal_vec
        //         .iter()
        //         .map(|x| x.output_name())
        //         .collect::<Vec<String>>()
        //         .join("\n"),
        // )
        // .block(Block::default().borders(Borders::ALL).title("Names"));

        let name_stamp_layouts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(9)].as_ref())
            .split(layouts[0]);

        let redundant = Paragraph::new(Line::from("RataWave").centered())
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(redundant, name_stamp_layouts[0]);

        let signal_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Max(4); signal_vec.len()])
            .split(layouts[1]);

        // let events = Layout::vertical(vec![Constraint::Min(3); signal_vec.len() * 2])
        //     .split(signal_layouts[1]);

        // let names = Layout::vertical(vec![Constraint::Min(3); signal_vec.len()]);

        self.arr_size = signal_layouts[1].width as usize;

        let mut time_stamp_str = String::from("");

        // Show stamps after each 10 steps
        let show_split = 10;

        let mut time_stamp_graph = String::from("");

        let mut stamp_index = 0;
        while stamp_index < self.arr_size {
            let mut time_stamp = format!(
                "{}",
                self.time_start.clone() + stamp_index as u64 * self.time_step.time()
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
            Line::from(""),
            Line::from(time_stamp_str),
            Line::from(time_stamp_graph),
        ]);

        frame.render_widget(time_show, name_stamp_layouts[1]);

        // frame.render_widget(signals, signal_layouts[0]);

        for (index, signal) in signal_vec.iter().enumerate() {
            let par = signal
                .events_arr_in_range(self.time_start.time(), self.time_step.time(), self.arr_size)
                .iter()
                .map(|x| match x {
                    crate::signal::DisplayEvent::Value(value_display_event) => {
                        match value_display_event {
                            crate::signal::ValueDisplayEvent::ChangeEvent(value) => {
                                value.to_string()
                            }
                            crate::signal::ValueDisplayEvent::Stay(value) => value.to_string(),
                            _ => "T".to_string(),
                        }
                    }
                    crate::signal::DisplayEvent::Vector(vector_display_event) => {
                        match vector_display_event {
                            crate::signal::VectorDisplayEvent::ChangeEvent(value) => {
                                value.to_string()
                            }
                            crate::signal::VectorDisplayEvent::Stay(value) => value.to_string(),
                            _ => "T".to_string(),
                        }
                    }
                })
                .collect::<String>();

            let mut signal_event_lines = self.get_lines_from_a_signal(signal);
            signal_event_lines.insert(0, Line::from(par));

            let sparkline = Paragraph::new(signal_event_lines);

            let a_signal_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Fill(1), Constraint::Fill(9)])
                .split(signal_layouts[index]);

            let a_signal_name = Line::from(signal_vec.get(index).unwrap().output_name());

            frame.render_widget(a_signal_name, a_signal_layout[0]);
            frame.render_widget(sparkline, a_signal_layout[1]);
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
                    self.time_step.step_decrease();
                }
                KeyCode::Char('-') => {
                    self.time_step.step_increase();
                }
                KeyCode::Char('h') => {
                    self.time_start
                        .decrease(self.arr_size as u64 / 2 * self.time_step.time());
                }
                KeyCode::Char('l') => {
                    self.time_start
                        .increase(self.arr_size as u64 / 2 * self.time_step.time());
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn get_lines_from_a_signal(&self, signal: &Signal) -> Vec<Line> {
        let display_event_arr = signal.events_arr_in_range(
            self.time_start.time(),
            self.time_step.time(),
            self.arr_size,
        );

        let lines =
            display_event_arr
                .iter()
                .fold(("".to_string(), "".to_string()), |mut lines, event| {
                    match event {
                        crate::signal::DisplayEvent::Value(value_display_event) => {
                            match value_display_event {
                                crate::signal::ValueDisplayEvent::ChangeEvent(value) => match value
                                {
                                    Value::V1 => {
                                        lines.0.push_str(RISING_EDGE.first_line);
                                        lines.1.push_str(RISING_EDGE.second_line);
                                    }
                                    Value::V0 => {
                                        lines.0.push_str(FALLING_EDGE.first_line);
                                        lines.1.push_str(FALLING_EDGE.second_line);
                                    }
                                    Value::X => {
                                        lines.0.push_str("x");
                                        lines.1.push_str("x");
                                    }
                                    Value::Z => {
                                        lines.0.push_str("z");
                                        lines.1.push_str("z");
                                    }
                                },
                                crate::signal::ValueDisplayEvent::MultipleEvent => {
                                    lines.0.push_str("␩");
                                    lines.1.push_str("␩");
                                    // lines.0.push_str("␨");
                                    // lines.1.push_str("␨");
                                }
                                crate::signal::ValueDisplayEvent::Stay(value) => match value {
                                    Value::V1 => {
                                        lines.0.push_str("─");
                                        lines.1.push_str(" ");
                                    }
                                    Value::V0 => {
                                        lines.0.push_str(" ");
                                        lines.1.push_str("─");
                                    }
                                    Value::X => {
                                        lines.0.push_str("x");
                                        lines.1.push_str("x");
                                    }
                                    Value::Z => {
                                        lines.0.push_str("z");
                                        lines.1.push_str("z");
                                    }
                                },
                            }
                        }
                        crate::signal::DisplayEvent::Vector(vector_display_event) => {
                            match vector_display_event {
                                _ => {
                                    lines.0.push_str("m");
                                    lines.0.push_str("m");
                                }
                            }
                        }
                    }

                    lines
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
