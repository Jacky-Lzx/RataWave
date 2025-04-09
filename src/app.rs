use crate::{
    modules::{
        module::Module,
        signal::{DisplayEvent, Signal, ValueDisplayEvent, VectorDisplayEvent},
        time::Time,
    },
    ui::{
        M_CHANGE, M_MULTIPLE, M_STAY, S_FALLING_EDGE, S_MULTIPLE, S_RISING_EDGE, S_STAY_0,
        S_STAY_1, S_STAY_X, S_STAY_Z,
    },
    utils::{middle_str, parse_files, vector_contain_x_or_z},
};

use std::{
    cmp::min,
    io::{self},
    rc::Rc,
};

use cli_log::debug;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{self, Block, Borders, Paragraph},
};
use std::str::FromStr;
use tui_textarea::TextArea;
use vcd::{Value, Vector};

#[derive(PartialEq)]
enum AppMode {
    Run,
    Input,
    Exit,
    AddSignal,
}

pub struct App<'a> {
    module_root: Module,
    time_start: Time,
    time_step: Time,
    arr_size: usize,
    // time_scale: TimescaleUnit,
    mode: AppMode,
    textarea: TextArea<'a>,
}

impl<'a> App<'a> {
    pub fn default() -> io::Result<Self> {
        let (module_root, time_base_scale) =
            parse_files(String::from("./assets/verilog/test_1.vcd"))?;
        debug!("Root: {}", module_root);
        Ok(Self {
            mode: AppMode::Run,
            module_root,
            time_start: Time::new(0, time_base_scale),
            time_step: Time::new(10, time_base_scale),
            arr_size: 100,
            textarea: TextArea::default(),
        })
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?
            }
            _ => {}
        };
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let main_layouts = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(frame.area());

        let signals = self.module_root.get_signals();

        let name_stamp_layouts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(9)].as_ref())
            .split(main_layouts[0]);

        let signal_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Max(4); signals.len()])
            .split(main_layouts[1]);

        let signal_layouts: Vec<Rc<[Rect]>> = signal_layouts
            .iter()
            .map(|&x| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Fill(1), Constraint::Fill(9)])
                    .split(x)
            })
            .collect();

        self.arr_size = signal_layouts[0][1].width as usize;

        // Display program title
        let redundant = Paragraph::new(Line::from("RataWave").centered())
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(redundant, name_stamp_layouts[0]);

        // Display time stamp
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

        // Display signals
        for (index, &signal) in signals.iter().enumerate() {
            let mut signal_event_lines = self.get_lines_from_a_signal(signal);
            signal_event_lines.insert(0, Line::from(self.get_value_string_from_a_signal(signal)));

            let signal_graph = Paragraph::new(signal_event_lines);

            let signal_name = Line::from(signals.get(index).unwrap().output_name());

            frame.render_widget(signal_name, signal_layouts[index][0]);
            frame.render_widget(signal_graph, signal_layouts[index][1]);
        }

        if self.mode == AppMode::Input {
            let color_green = (*catppuccin::PALETTE
                .mocha
                .get_color(catppuccin::ColorName::Green))
            .into();
            let color_red = (*catppuccin::PALETTE
                .mocha
                .get_color(catppuccin::ColorName::Red))
            .into();

            let color_text = (*catppuccin::PALETTE
                .mocha
                .get_color(catppuccin::ColorName::Text))
            .into();

            let input = &self.textarea.lines()[0];

            match Time::is_valid(input) {
                Ok(_) => {
                    self.textarea.set_style(Style::default().fg(color_green));
                    self.textarea.set_block(
                        Block::default()
                            .border_style(color_green)
                            .borders(Borders::ALL)
                            .title("Enter a time (e.g. 100ns) [Valid]"),
                    );
                }
                Err(e) => {
                    if input.len() == 0 {
                        self.textarea.set_style(Style::default().fg(color_text));
                        self.textarea.set_block(
                            Block::default()
                                .border_style(color_text)
                                .borders(Borders::ALL)
                                .title(format!("Enter a time (e.g. 100ns)")),
                        );
                    } else {
                        self.textarea.set_style(Style::default().fg(color_red));
                        self.textarea.set_block(
                            Block::default()
                                .border_style(color_red)
                                .borders(Borders::ALL)
                                .title(format!(
                                    "Enter a time (e.g. 100ns) [Invalid: {}]",
                                    e.message()
                                )),
                        );
                    }
                }
            };

            let vertical = Layout::vertical([Constraint::Max(3)]).flex(Flex::Start);
            let horizontal = Layout::horizontal([Constraint::Max(80)]).flex(Flex::Center);
            let [area] = vertical.areas(frame.area());
            let [area] = horizontal.areas(area);
            frame.render_widget(widgets::Clear, area); //this clears out the background
            frame.render_widget(&self.textarea, area);
        } else if self.mode == AppMode::AddSignal {
            let vertical = Layout::vertical([Constraint::Max(30)]).flex(Flex::Center);
            let horizontal = Layout::horizontal([Constraint::Max(80)]).flex(Flex::Center);
            let [area] = vertical.areas(frame.area());
            let [area] = horizontal.areas(area);
            frame.render_widget(widgets::Clear, area); //this clears out the background
            let par = Paragraph::new("").block(Block::default().borders(Borders::ALL));
            frame.render_widget(par, area);
        }
    }

    fn handle_key_event(&mut self, key_event: event::KeyEvent) -> io::Result<()> {
        match self.mode {
            AppMode::Run => match key_event.code {
                KeyCode::Char('a') => {
                    self.mode = AppMode::AddSignal;
                }
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
                KeyCode::Char('t') => {
                    self.mode = AppMode::Input;
                    // Initialize textarea
                    self.textarea = TextArea::default();
                }
                _ => {}
            },

            AppMode::Input => match key_event.code {
                // When pressing Esc, directly return to the normal mode
                KeyCode::Esc => {
                    self.mode = AppMode::Run;
                }
                KeyCode::Enter => {
                    if Time::is_valid(self.textarea.lines()[0].as_str()).is_ok() {
                        self.mode = AppMode::Run;
                        let text = self.textarea.lines(); // Get input text
                        let text = text.first().unwrap();
                        let time = Time::from_str(text).unwrap();
                        self.time_start = time;
                    }
                }
                _ => {
                    self.textarea.input(key_event);
                }
            },
            AppMode::AddSignal => match key_event.code {
                KeyCode::Esc => {
                    self.mode = AppMode::Run;
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn get_value_string_from_a_signal(&self, signal: &Signal) -> String {
        signal
            .events_arr_in_range(self.time_start.time(), self.time_step.time(), self.arr_size)
            .iter()
            .map(|x| match x {
                DisplayEvent::Value(value_display_event) => match value_display_event {
                    ValueDisplayEvent::ChangeEvent(value) => value.to_string(),
                    ValueDisplayEvent::Stay(value) => value.to_string(),
                    _ => "T".to_string(),
                },
                DisplayEvent::Vector(vector_display_event) => match vector_display_event {
                    VectorDisplayEvent::ChangeEvent(value) => value.to_string(),
                    VectorDisplayEvent::Stay(value) => value.to_string(),
                    _ => "T".to_string(),
                },
            })
            .collect::<String>()
    }

    fn get_lines_from_a_signal(&self, signal: &Signal) -> Vec<Line> {
        let display_event_arr = signal.events_arr_in_range(
            self.time_start.time(),
            self.time_step.time(),
            self.arr_size,
        );

        let color_green = (*catppuccin::PALETTE
            .mocha
            .get_color(catppuccin::ColorName::Green))
        .into();
        let color_red = (*catppuccin::PALETTE
            .mocha
            .get_color(catppuccin::ColorName::Red))
        .into();

        let mut lines = display_event_arr.iter().fold(vec![], |mut lines, event| {
            if lines.len() == 0 {
                lines = match event {
                    DisplayEvent::Value(_) => vec![vec![]; 2],
                    DisplayEvent::Vector(_) => vec![vec![]; 3],
                };
            }

            match event {
                DisplayEvent::Value(value_display_event) => {
                    let (symbols, color) = match value_display_event {
                        ValueDisplayEvent::ChangeEvent(value) => {
                            let symbols = match value {
                                Value::V0 => S_FALLING_EDGE,
                                Value::V1 => S_RISING_EDGE,
                                Value::X => S_STAY_X,
                                Value::Z => S_STAY_Z,
                            };
                            (symbols, color_green)
                        }
                        ValueDisplayEvent::Stay(value) => {
                            let symbols = match value {
                                Value::V0 => S_STAY_0,
                                Value::V1 => S_STAY_1,
                                Value::X => S_STAY_X,
                                Value::Z => S_STAY_Z,
                            };
                            (symbols, color_green)
                        }
                        ValueDisplayEvent::MultipleEvent => (S_MULTIPLE, color_green),
                    };
                    lines.iter_mut().enumerate().for_each(|(i, x)| {
                        x.push(Span::styled(symbols[i], Style::default().fg(color)));
                    });
                }
                DisplayEvent::Vector(vector_display_event) => {
                    let (symbols, color) = match vector_display_event {
                        VectorDisplayEvent::ChangeEvent(_) => (M_CHANGE, color_green),
                        VectorDisplayEvent::Stay(vector) => {
                            let color = match vector_contain_x_or_z(vector) {
                                true => color_red,
                                false => color_green,
                            };
                            (M_STAY, color)
                        }
                        VectorDisplayEvent::MultipleEvent => (M_MULTIPLE, color_green),
                    };
                    lines.iter_mut().enumerate().for_each(|(i, x)| {
                        x.push(Span::styled(symbols[i], Style::default().fg(color)));
                    });
                }
            };

            lines
        });

        // Show binary values for Vector signals in the middle line
        let mut start_index = None;
        let mut vector_value: Option<Vector> = None;
        display_event_arr
            .iter()
            .enumerate()
            .for_each(|(i, event)| match event {
                DisplayEvent::Value(_) => {}
                DisplayEvent::Vector(vector_display_event) => match vector_display_event {
                    VectorDisplayEvent::ChangeEvent(vector) => {
                        match start_index {
                            Some(index) => {
                                lines[1].splice(
                                    index + 1..i,
                                    middle_str(
                                        i - index - 1,
                                        vector_value.clone().unwrap().to_string(),
                                    )
                                    .into_iter(),
                                );
                            }
                            None => {}
                        };
                        start_index = Some(i);
                        vector_value = Some(vector.clone());
                    }
                    VectorDisplayEvent::MultipleEvent => {}
                    VectorDisplayEvent::Stay(vector) => match start_index {
                        None => {
                            start_index = Some(i);
                            vector_value = Some(vector.clone());
                        }
                        _ => {}
                    },
                },
            });

        // Last vector
        if let Some(index) = start_index {
            use VectorDisplayEvent::*;
            match &display_event_arr[index] {
                DisplayEvent::Vector(ChangeEvent(_)) | DisplayEvent::Vector(Stay(_)) => {
                    let len = lines[1].len();
                    lines[1].splice(
                        index + 1..len,
                        middle_str(len - index - 1, vector_value.unwrap().to_string()).into_iter(),
                    );
                }
                _ => {}
            };
        };

        lines.into_iter().map(|x| Line::from(x)).collect::<Vec<_>>()
    }
}

impl<'a> App<'a> {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.mode != AppMode::Exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }
}
