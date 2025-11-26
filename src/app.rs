use crate::{
    modules::{
        cli_args::CliArgs,
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
    cell::RefCell,
    cmp::{max, min},
    io::{self},
    ops::Deref,
    rc::Rc,
};

use cli_log::debug;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{self, Block, Borders, List, ListItem, Paragraph},
};
use std::str::FromStr;
use tui_textarea::TextArea;
use vcd::{Value, Vector};

#[derive(PartialEq)]
enum AppMode<'a> {
    Run(Box<RunAssets>),
    Input(Box<InputAssets<'a>>),
    Exit,
    SignalAdd(Box<SignalAddAssets>),
}

#[derive(Default)]
struct RunAssets {
    // None if there are no signal
    choice_index: Option<usize>,
    expended_signals: Vec<Rc<RefCell<Signal>>>,
}

impl PartialEq for RunAssets {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(PartialEq, Default)]
struct SignalAddAssets {
    // None if there are no signal
    choice_index: Option<usize>,
}

#[derive(Default)]
struct InputAssets<'a> {
    textarea: TextArea<'a>,
    // None if there are no signal
    choice_index: Option<usize>,
}

impl PartialEq for InputAssets<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.choice_index == other.choice_index
    }
}

pub struct App<'a> {
    module_root: Rc<RefCell<Module>>,
    signals: Vec<Rc<RefCell<Signal>>>,
    displayed_signals: Vec<Rc<RefCell<Signal>>>,
    undisplayed_signals: Vec<Rc<RefCell<Signal>>>,
    time_start: Time,
    time_step: Time,
    arr_size: usize,
    // time_scale: TimescaleUnit,
    mode: AppMode<'a>,
}

fn filter_displayed_signals(
    all_signals: &[Rc<RefCell<Signal>>],
    displayed_signals: &[Rc<RefCell<Signal>>],
) -> Vec<Rc<RefCell<Signal>>> {
    all_signals
        .iter()
        .filter(|e2| !displayed_signals.iter().any(|e1| Rc::ptr_eq(e1, e2)))
        .cloned()
        .collect()
}

impl<'a> App<'a> {
    pub fn default(cli_args: CliArgs) -> io::Result<Self> {
        let (module_root, time_base_scale) = parse_files(cli_args.file_path)?;
        debug!("Root: {}", module_root.borrow());

        let signals = module_root.borrow().get_signals();
        let undisplayed_signals = filter_displayed_signals(&signals, &[]);

        Ok(Self {
            mode: AppMode::SignalAdd(Box::default()),
            module_root,
            signals,
            displayed_signals: vec![],
            undisplayed_signals,
            time_start: Time::zero(),
            time_step: Time::new(10, time_base_scale),
            arr_size: 100,
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

        let name_stamp_layouts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(9)].as_ref())
            .split(main_layouts[0]);

        let signal_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Max(3);
                // FIXME: if displayed_signals = 0, it will crash, so adding a max here
                max(1, self.displayed_signals.len())
            ])
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
            let mut time_stamp = format!("{}", self.time_start + self.time_step * stamp_index);
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
        for (index, signal) in self.displayed_signals.iter().enumerate() {
            let signal = signal.borrow();

            if let AppMode::Run(assets) = &mut self.mode
                && assets.choice_index.is_none()
                && !self.displayed_signals.is_empty()
            {
                assets.choice_index = Some(0);
            }

            let signal_event_lines = self.get_lines_from_a_signal(&signal);
            // signal_event_lines.insert(0, Line::from(self.get_value_string_from_a_signal(&signal)));

            let signal_graph = Paragraph::new(signal_event_lines);

            let selected_signal_style = if let AppMode::Run(assets) = &self.mode {
                if assets.choice_index.is_some_and(|x| x == index) {
                    Style::default().bg(Color::DarkGray)
                } else if assets
                    .expended_signals
                    .iter()
                    .any(|x| Rc::ptr_eq(x, &self.displayed_signals[index]))
                {
                    Style::default().bg(Color::Gray)
                } else {
                    Style::default()
                }
            } else {
                Style::default()
            };
            let signal_name = Paragraph::new(
                Line::from(
                    self.displayed_signals
                        .get(index)
                        .unwrap()
                        .borrow()
                        .output_name(),
                )
                .centered(),
            )
            .block(Block::default().borders(Borders::TOP))
            .style(selected_signal_style);

            frame.render_widget(signal_name, signal_layouts[index][0]);
            frame.render_widget(signal_graph, signal_layouts[index][1]);
        }

        match &mut self.mode {
            AppMode::Input(assets) => {
                let assets = assets.as_mut();

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

                let input = &assets.textarea.lines()[0];

                match Time::is_valid(input) {
                    Ok(_) => {
                        assets.textarea.set_style(Style::default().fg(color_green));
                        assets.textarea.set_block(
                            Block::default()
                                .border_style(color_green)
                                .borders(Borders::ALL)
                                .title("Enter a time (e.g. 100ns) [Valid]"),
                        );
                    }
                    Err(e) => {
                        if input.is_empty() {
                            assets.textarea.set_style(Style::default().fg(color_text));
                            assets.textarea.set_block(
                                Block::default()
                                    .border_style(color_text)
                                    .borders(Borders::ALL)
                                    .title("Enter a time (e.g. 100ns)".to_string()),
                            );
                        } else {
                            assets.textarea.set_style(Style::default().fg(color_red));
                            assets.textarea.set_block(
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
                frame.render_widget(&assets.textarea, area);
            }
            AppMode::SignalAdd(assets) => {
                let assets = assets.as_mut();

                if assets.choice_index.is_none() && !self.undisplayed_signals.is_empty() {
                    assets.choice_index = Some(0);
                }

                let vertical = Layout::vertical([Constraint::Max(30)]).flex(Flex::Center);
                let horizontal = Layout::horizontal([Constraint::Max(80)]).flex(Flex::Center);
                let [area] = vertical.areas(frame.area());
                let [area] = horizontal.areas(area);
                frame.render_widget(widgets::Clear, area); //this clears out the background

                let undisplayed_signals: Vec<Span> = self
                    .undisplayed_signals
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        Span::styled(
                            x.borrow().output_path().clone(),
                            if i == assets.choice_index.unwrap() {
                                Style::default().fg(Color::Blue)
                            } else {
                                Style::default()
                            },
                        )
                    })
                    .collect();
                let lines: Vec<Line> = undisplayed_signals
                    .iter()
                    .map(|x| Line::from(x.clone()))
                    .collect();
                let par = Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title_top("Add signals, press 'a' to add all, press 'q' to exit"),
                );
                frame.render_widget(par, area);
            }
            _ => {}
        }
    }

    fn handle_key_event(&mut self, key_event: event::KeyEvent) -> io::Result<()> {
        let mut new_mode = None;

        match &mut self.mode {
            AppMode::Run(assets) => match key_event.code {
                KeyCode::Char('a') => {
                    self.mode = AppMode::SignalAdd(Box::default());
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
                    self.time_start = self.time_step * (self.arr_size / 2);
                }
                KeyCode::Char('l') => {
                    self.time_start += self.time_step * (self.arr_size / 2);
                }
                KeyCode::Char(' ') => {
                    let assets = assets.as_mut();
                    if let Some(index) = assets.choice_index {
                        if self.displayed_signals.is_empty() {
                            return Ok(());
                        }
                        let signal = self.displayed_signals[index].clone();

                        if !signal.borrow().deref().is_vector() {
                            return Ok(());
                        }

                        if assets
                            .expended_signals
                            .iter()
                            .any(|x| Rc::ptr_eq(x, &signal))
                        {
                            assets.expended_signals.retain(|x| !Rc::ptr_eq(x, &signal));
                        } else {
                            assets.expended_signals.push(signal);
                        }
                    }
                }
                KeyCode::Char('j') => {
                    let assets = assets.as_mut();
                    assets.choice_index = if self.displayed_signals.is_empty() {
                        None
                    } else {
                        Some(min(
                            assets.choice_index.unwrap_or_default() + 1,
                            self.displayed_signals.len() - 1,
                        ))
                    };
                }
                KeyCode::Char('k') => {
                    let assets = assets.as_mut();
                    assets.choice_index =
                        Some(assets.choice_index.unwrap_or_default().saturating_sub(1));
                }
                // Deleted the selected signal
                KeyCode::Char('d') => {
                    if self.displayed_signals.is_empty() {
                        return Ok(());
                    }

                    let assets = assets.as_mut();
                    let signal = self
                        .displayed_signals
                        .remove(assets.choice_index.unwrap_or_default());
                    self.undisplayed_signals.push(signal);
                    if !self.displayed_signals.is_empty() {
                        assets.choice_index = Some(min(
                            assets.choice_index.unwrap_or_default(),
                            self.displayed_signals.len() - 1,
                        ))
                    }
                }
                KeyCode::Char('t') => {
                    self.mode = AppMode::Input(Box::default());
                }
                _ => {}
            },

            AppMode::Input(assets) => match key_event.code {
                // When pressing Esc, directly return to the normal mode
                KeyCode::Esc => {
                    self.mode = AppMode::Run(Box::default());
                }
                KeyCode::Enter => {
                    if Time::is_valid(assets.textarea.lines()[0].as_str()).is_ok() {
                        new_mode = Some(AppMode::Run(Box::default()));
                        let text = assets.textarea.lines(); // Get input text
                        let text = text.first().unwrap();
                        let time = Time::from_str(text).unwrap();
                        self.time_start = time;
                    }
                }
                _ => {
                    assets.textarea.input(key_event);
                }
            },
            AppMode::SignalAdd(assets) => {
                let assets = assets.as_mut();

                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.mode = AppMode::Run(Box::default());
                    }
                    KeyCode::Char('a') => {
                        self.mode = AppMode::Run(Box::default());
                        // Add all signals to displayed signals
                        for signal in &self.undisplayed_signals {
                            self.displayed_signals.push(Rc::clone(signal));
                        }
                        self.undisplayed_signals.clear();
                    }
                    KeyCode::Char('j') => {
                        assets.choice_index = if self.undisplayed_signals.is_empty() {
                            None
                        } else {
                            Some(min(
                                assets.choice_index.unwrap_or_default() + 1,
                                self.undisplayed_signals.len() - 1,
                            ))
                        }
                    }
                    KeyCode::Char('k') => {
                        assets.choice_index = assets.choice_index.map(|x| x.saturating_sub(1));
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if self.undisplayed_signals.is_empty() {
                            return Ok(());
                        }

                        self.displayed_signals.push(Rc::clone(
                            self.undisplayed_signals
                                .get(assets.choice_index.unwrap_or_default())
                                .unwrap(),
                        ));
                        self.undisplayed_signals
                            .remove(assets.choice_index.unwrap_or_default());
                        if !self.undisplayed_signals.is_empty() {
                            assets.choice_index = Some(min(
                                assets.choice_index.unwrap_or_default(),
                                self.undisplayed_signals.len() - 1,
                            ))
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if let Some(mode) = new_mode {
            self.mode = mode;
        }

        Ok(())
    }

    fn get_value_string_from_a_signal(&self, signal: &Signal) -> String {
        signal
            .events_arr_in_range(self.time_start, self.time_step, self.arr_size)
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

    fn get_lines_from_a_signal(&'_ self, signal: &Signal) -> Vec<Line<'_>> {
        let display_event_arr =
            signal.events_arr_in_range(self.time_start, self.time_step, self.arr_size);

        let color_green = (*catppuccin::PALETTE
            .mocha
            .get_color(catppuccin::ColorName::Green))
        .into();
        let color_red = (*catppuccin::PALETTE
            .mocha
            .get_color(catppuccin::ColorName::Red))
        .into();

        let mut lines = display_event_arr.iter().fold(vec![], |mut lines, event| {
            if lines.is_empty() {
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
                        if let Some(index) = start_index {
                            lines[1].splice(
                                index + 1..i,
                                middle_str(
                                    i - index - 1,
                                    vector_value.clone().unwrap().to_string(),
                                )
                                .into_iter(),
                            );
                        };
                        start_index = Some(i);
                        vector_value = Some(vector.clone());
                    }
                    VectorDisplayEvent::MultipleEvent => {}
                    VectorDisplayEvent::Stay(vector) => {
                        if start_index.is_none() {
                            start_index = Some(i);
                            vector_value = Some(vector.clone());
                        }
                    }
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

        lines.into_iter().map(Line::from).collect::<Vec<_>>()
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
