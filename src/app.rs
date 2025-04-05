use std::{
    cmp::{max, min},
    fmt::Display,
    fs::File,
    io::{self, BufReader},
    ops::Add,
    rc::Rc,
};

use cli_log::debug;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{self, Block, Borders, Paragraph},
};
use std::str::FromStr;
use tui_textarea::TextArea;
use vcd::{ScopeItem, TimescaleUnit, Value, Vector};

use crate::{
    Module, Signal,
    signal::{self, FALLING_EDGE, RISING_EDGE},
};

use crate::signal::ValueType;

#[derive(PartialEq)]
enum AppMode {
    Run,
    Input,
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

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

#[derive(Debug, PartialEq, Eq)]
struct ParseTimeError {
    message: String,
}

impl Display for ParseTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse time error: {}", self.message)
    }
}

impl FromStr for Time {
    type Err = ParseTimeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() == 0 {
            return Err(ParseTimeError {
                message: "Empty string".to_string(),
            });
        }

        let split_index: usize = s.find(|x: char| !x.is_numeric()).ok_or(ParseTimeError {
            message: "Split error".to_string(),
        })?;

        let (time, unit) = s.split_at(split_index);

        let time = time.parse::<u64>().map_err(|_| ParseTimeError {
            message: "Parse time error".to_string(),
        })?;
        let unit = TimescaleUnit::from_str(unit.trim()).map_err(|_| ParseTimeError {
            message: "Parse unit error".to_string(),
        })?;

        if unit == TimescaleUnit::FS {
            return Err(ParseTimeError {
                message: "Not support FS time scale.".to_string(),
            });
        }

        let time = time * TimescaleUnit::PS.divisor() / unit.divisor();

        Ok(Time { time })
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

    /// Check if the given string is a valid time
    /// E.g. "100ns" or "100 ns" is a valid time
    ///
    /// ```
    /// assert_eq!(is_valid("100ns"), true)
    /// ```
    pub fn is_valid(s: &str) -> Result<(), ParseTimeError> {
        match Time::from_str(s) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
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

fn middle_str<'a>(length: usize, mid_str: String) -> Vec<Span<'a>> {
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

fn vector_contain_x_or_z(vector: &Vector) -> bool {
    vector
        .iter()
        .find(|&x| x == Value::X || x == Value::Z)
        .iter()
        .count()
        != 0
}

impl<'a> App<'a> {
    pub fn default() -> io::Result<Self> {
        let (module_root, time_base_scale) = parse_files(String::from("./src/test_1.vcd"))?;
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
        for (index, signal) in signals.iter().enumerate() {
            let mut signal_event_lines = self.get_lines_from_a_signal(signal);
            signal_event_lines.insert(0, Line::from(self.get_value_string_from_a_signal(signal)));

            let signal_graph = Paragraph::new(signal_event_lines);

            let signal_name = Line::from(signals.get(index).unwrap().output_name());

            frame.render_widget(signal_name, signal_layouts[index][0]);
            frame.render_widget(signal_graph, signal_layouts[index][1]);
        }

        if self.mode == AppMode::Input {
            match Time::is_valid(&self.textarea.lines()[0]) {
                Ok(_) => {
                    self.textarea
                        .set_style(Style::default().fg(Color::LightGreen));
                    self.textarea.set_block(
                        Block::default()
                            .border_style(Color::LightGreen)
                            .borders(Borders::ALL)
                            .title("Valid time"),
                    );
                }
                Err(e) => {
                    self.textarea
                        .set_style(Style::default().fg(Color::LightRed));
                    self.textarea.set_block(
                        Block::default()
                            .border_style(Color::LightRed)
                            .borders(Borders::ALL)
                            .title(format!("{}! Please Enter a valid time like \"100ns\"", e)),
                    );
                }
            };
            // let block = Block::bordered().title("Popup");
            let area = popup_area(frame.area(), 60, 20);
            frame.render_widget(widgets::Clear, area); //this clears out the background
            frame.render_widget(&self.textarea, area);
        }
    }

    fn handle_key_event(&mut self, key_event: event::KeyEvent) -> io::Result<()> {
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
                KeyCode::Char('t') => {
                    self.mode = AppMode::Input;
                    // Initialize textarea
                    self.textarea = TextArea::default();
                    self.textarea
                        .set_style(Style::default().fg(Color::LightGreen));
                    self.textarea.set_block(
                        Block::default()
                            .border_style(Color::LightGreen)
                            .borders(Borders::ALL)
                            .title("Enter a time"),
                    );
                    self.textarea.set_cursor_line_style(Style::default());
                }
                _ => {}
            },

            AppMode::Input => match key_event.code {
                KeyCode::Esc | KeyCode::Enter => {
                    if Time::is_valid(self.textarea.lines()[0].as_str()).is_ok() {
                        self.mode = AppMode::Run;
                        let text = self.textarea.lines(); // Get input text
                        let text = text.first().unwrap();
                        debug!("input text: {:?}", text);
                        let time = Time::from_str(text).unwrap();
                        self.time_start = time;
                    }
                }
                _ => {
                    self.textarea.input(key_event);
                }
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
                crate::signal::DisplayEvent::Value(value_display_event) => {
                    match value_display_event {
                        crate::signal::ValueDisplayEvent::ChangeEvent(value) => value.to_string(),
                        crate::signal::ValueDisplayEvent::Stay(value) => value.to_string(),
                        _ => "T".to_string(),
                    }
                }
                crate::signal::DisplayEvent::Vector(vector_display_event) => {
                    match vector_display_event {
                        crate::signal::VectorDisplayEvent::ChangeEvent(value) => value.to_string(),
                        crate::signal::VectorDisplayEvent::Stay(value) => value.to_string(),
                        _ => "T".to_string(),
                    }
                }
            })
            .collect::<String>()
    }

    fn get_lines_from_a_signal(&self, signal: &Signal) -> Vec<Line> {
        let display_event_arr = signal.events_arr_in_range(
            self.time_start.time(),
            self.time_step.time(),
            self.arr_size,
        );

        let color_green = Color::from_str("#a6e3a1").unwrap();
        let color_red = Color::from_str("#f38ba8").unwrap();

        let mut lines = display_event_arr.iter().fold(vec![], |mut lines, event| {
            match event {
                crate::signal::DisplayEvent::Value(value_display_event) => {
                    if lines.len() == 0 {
                        lines.push(vec![]);
                        lines.push(vec![]);
                    }
                    let [mut line0, mut line1] = lines.try_into().unwrap();

                    match value_display_event {
                        crate::signal::ValueDisplayEvent::ChangeEvent(value) => match value {
                            Value::V1 => {
                                line0.push(Span::styled(
                                    RISING_EDGE.first_line,
                                    Style::default().fg(color_green),
                                ));
                                line1.push(Span::styled(
                                    RISING_EDGE.second_line,
                                    Style::default().fg(color_green),
                                ));
                            }
                            Value::V0 => {
                                line0.push(Span::styled(
                                    FALLING_EDGE.first_line,
                                    Style::default().fg(color_green),
                                ));
                                line1.push(Span::styled(
                                    FALLING_EDGE.second_line,
                                    Style::default().fg(color_green),
                                ));
                            }
                            Value::X => {
                                line0.push(Span::styled("x", Style::default().fg(color_red)));
                                line1.push(Span::styled("x", Style::default().fg(color_red)));
                            }
                            Value::Z => {
                                line0.push(Span::styled("z", Style::default().fg(color_red)));
                                line1.push(Span::styled("z", Style::default().fg(color_red)));
                            }
                        },
                        crate::signal::ValueDisplayEvent::MultipleEvent => {
                            line0.push(Span::styled("␩", Style::default().fg(color_green)));
                            line1.push(Span::styled("␩", Style::default().fg(color_green)));
                            // line1.push(Span::styled("␨", Style::default().fg(color_green)));
                            // line1.push(Span::styled("␨", Style::default().fg(color_green)));
                        }
                        crate::signal::ValueDisplayEvent::Stay(value) => match value {
                            Value::V1 => {
                                line0.push(Span::styled("─", Style::default().fg(color_green)));
                                line1.push(Span::styled(" ", Style::default().fg(color_green)));
                            }
                            Value::V0 => {
                                line0.push(Span::styled(" ", Style::default().fg(color_green)));
                                line1.push(Span::styled("─", Style::default().fg(color_green)));
                            }
                            Value::X => {
                                line0.push(Span::styled("x", Style::default().fg(color_red)));
                                line1.push(Span::styled("x", Style::default().fg(color_red)));
                            }
                            Value::Z => {
                                line0.push(Span::styled("z", Style::default().fg(color_red)));
                                line1.push(Span::styled("z", Style::default().fg(color_red)));
                            }
                        },
                    }
                    vec![line0, line1]
                }
                crate::signal::DisplayEvent::Vector(vector_display_event) => {
                    if lines.len() == 0 {
                        lines.push(vec![]);
                        lines.push(vec![]);
                        lines.push(vec![]);
                    }
                    let [mut line0, mut line1, mut line2] = lines.try_into().unwrap();

                    match vector_display_event {
                        signal::VectorDisplayEvent::ChangeEvent(_) => {
                            line0.push(Span::styled("┬", Style::default().fg(color_green)));
                            line1.push(Span::styled("│", Style::default().fg(color_green)));
                            line2.push(Span::styled("┴", Style::default().fg(color_green)));
                        }
                        signal::VectorDisplayEvent::MultipleEvent => {
                            line0.push(Span::styled("␩", Style::default().fg(color_green)));
                            line1.push(Span::styled("␩", Style::default().fg(color_green)));
                            line2.push(Span::styled("␩", Style::default().fg(color_green)));
                        }
                        signal::VectorDisplayEvent::Stay(vector) => {
                            let color = match vector_contain_x_or_z(vector) {
                                true => color_red,
                                false => color_green,
                            };
                            line0.push(Span::styled("─", Style::default().fg(color)));
                            line1.push(Span::styled(" ", Style::default().fg(color)));
                            line2.push(Span::styled("─", Style::default().fg(color)));
                        }
                    }
                    vec![line0, line1, line2]
                }
            }
        });

        // Show binary values for Vector signals in the middle line
        let mut start_index = None;
        let mut vector_value: Option<Vector> = None;
        display_event_arr
            .iter()
            .enumerate()
            .for_each(|(i, event)| match event {
                signal::DisplayEvent::Value(_) => {}
                signal::DisplayEvent::Vector(vector_display_event) => match vector_display_event {
                    signal::VectorDisplayEvent::ChangeEvent(vector) => {
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
                    signal::VectorDisplayEvent::MultipleEvent => {}
                    signal::VectorDisplayEvent::Stay(vector) => match start_index {
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
            use signal::VectorDisplayEvent::*;
            match &display_event_arr[index] {
                signal::DisplayEvent::Vector(ChangeEvent(_))
                | signal::DisplayEvent::Vector(Stay(_)) => {
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
