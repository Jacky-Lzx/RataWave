use std::{cell::RefCell, rc::Rc};

use crate::{
    ui::{
        M_CHANGE, M_MULTIPLE, M_STAY, S_FALLING_EDGE, S_MULTIPLE, S_RISING_EDGE, S_STAY_0,
        S_STAY_1, S_STAY_X, S_STAY_Z,
    },
    utils::{middle_str, vector_contain_x_or_z},
};

use ratatui::{
    style::{Style, Styled},
    text::{Line, Span, Text},
    widgets::ListItem,
};
use vcd::{Value, Vector};

use crate::{
    signal::{DisplayEvent, Signal, ValueDisplayEvent, VectorDisplayEvent},
    time::Time,
};

pub trait DisplayedSignalTrait {
    fn get_name_items(&self) -> Vec<ListItem<'_>>;
    fn get_value_string(&self, time_start: Time, time_step: Time, arr_size: usize) -> String;
    fn get_lines(&self, time_start: Time, time_step: Time, arr_size: usize) -> Vec<Line<'_>>;
    fn get_graph_items(
        &self,
        time_start: Time,
        time_step: Time,
        arr_size: usize,
    ) -> Vec<ListItem<'_>>;
    fn get_name(&self) -> String;
    fn get_path(&self) -> String;
}

pub enum DisplayedSignal {
    Vector(DisplayedVector),
    Value(DisplayedValue),
}

pub struct DisplayedVector {
    signal: Rc<RefCell<Signal>>,
    is_expand: bool,
}

pub struct DisplayedValue {
    signal: Rc<RefCell<Signal>>,
}

impl From<Rc<RefCell<Signal>>> for DisplayedSignal {
    fn from(signal: Rc<RefCell<Signal>>) -> Self {
        match signal.borrow().is_vector() {
            true => DisplayedSignal::Vector(DisplayedVector {
                signal: signal.clone(),
                is_expand: false,
            }),
            false => DisplayedSignal::Value(DisplayedValue {
                signal: signal.clone(),
            }),
        }
    }
}

impl DisplayedSignalTrait for DisplayedSignal {
    fn get_value_string(&self, time_start: Time, time_step: Time, arr_size: usize) -> String {
        let data = match self {
            DisplayedSignal::Vector(v) => v.signal.clone(),
            DisplayedSignal::Value(v) => v.signal.clone(),
        };

        let events = data
            .borrow()
            .events_arr_in_range(time_start, time_step, arr_size);

        events
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

    fn get_lines(&self, time_start: Time, time_step: Time, arr_size: usize) -> Vec<Line<'_>> {
        let signal = match self {
            DisplayedSignal::Vector(v) => &v.signal.clone(),
            DisplayedSignal::Value(v) => &v.signal.clone(),
        };

        let events = signal
            .borrow()
            .events_arr_in_range(time_start, time_step, arr_size);

        let color_green = (*catppuccin::PALETTE
            .mocha
            .get_color(catppuccin::ColorName::Green))
        .into();
        let color_red = (*catppuccin::PALETTE
            .mocha
            .get_color(catppuccin::ColorName::Red))
        .into();

        let mut lines = events.iter().fold(vec![], |mut lines, event| {
            if lines.is_empty() {
                lines = match event {
                    DisplayEvent::Value(_) => vec![vec![]; 3],
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
        events
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
                                ),
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
            match &events[index] {
                DisplayEvent::Vector(ChangeEvent(_)) | DisplayEvent::Vector(Stay(_)) => {
                    let len = lines[1].len();
                    lines[1].splice(
                        index + 1..len,
                        middle_str(len - index - 1, vector_value.unwrap().to_string()),
                    );
                }
                _ => {}
            };
        };

        lines.into_iter().map(Line::from).collect::<Vec<_>>()
    }

    fn get_name(&self) -> String {
        match self {
            DisplayedSignal::Vector(v) => v.signal.borrow().output_name(),
            DisplayedSignal::Value(v) => v.signal.borrow().output_name(),
        }
    }

    fn get_path(&self) -> String {
        match self {
            DisplayedSignal::Vector(v) => v.signal.borrow().output_path(),
            DisplayedSignal::Value(v) => v.signal.borrow().output_path(),
        }
    }

    fn get_graph_items(
        &self,
        time_start: Time,
        time_step: Time,
        arr_size: usize,
    ) -> Vec<ListItem<'_>> {
        let style = if self.is_expanded() {
            Style::default().fg((*catppuccin::PALETTE
                .mocha
                .get_color(catppuccin::ColorName::Pink))
            .into())
        } else {
            Style::default()
        };

        let item = ListItem::from(self.get_lines(time_start, time_step, arr_size)).style(style);

        vec![item]
    }

    fn get_name_items(&self) -> Vec<ListItem<'_>> {
        let style = if self.is_expanded() {
            Style::default().fg((*catppuccin::PALETTE
                .mocha
                .get_color(catppuccin::ColorName::Pink))
            .into())
        } else {
            Style::default()
        };

        let signal_name = ListItem::new(Text::from(vec![
            Line::from("\n"),
            Line::from(self.get_name()).centered(),
            Line::from("\n"),
        ]))
        .style(style);

        vec![signal_name]
    }
}

impl DisplayedSignal {
    pub fn toggle_expand(&mut self) -> Result<(), String> {
        match self {
            DisplayedSignal::Vector(v) => {
                v.is_expand = !v.is_expand;
                Ok(())
            }
            DisplayedSignal::Value(_) => Err("Value can not be expanded".to_string()),
        }
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            DisplayedSignal::Vector(v) => v.is_expand,
            DisplayedSignal::Value(_) => false,
        }
    }
}
