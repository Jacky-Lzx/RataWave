use core::{fmt, panic};
use std::{cell::RefCell, fmt::Display, rc::Weak};

use vcd::{IdCode, Value, Var, Vector};

use super::module::Module;

/// Type of the signal
/// - `Value`: the signal has only one bit
/// - `Vector`: the signal has multiple bits
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Value(Value),
    Vector(Vector),
}

#[derive(Clone, Debug)]
pub enum ValueDisplayEvent {
    ChangeEvent(Value),
    MultipleEvent,
    Stay(Value),
}

#[derive(Clone, Debug)]
pub enum VectorDisplayEvent {
    ChangeEvent(Vector),
    MultipleEvent,
    Stay(Vector),
}

#[derive(Clone, Debug)]
pub enum DisplayEvent {
    Value(ValueDisplayEvent),
    Vector(VectorDisplayEvent),
}

impl PartialEq<ValueType> for DisplayEvent {
    fn eq(&self, other: &ValueType) -> bool {
        match other {
            ValueType::Value(value) => match self {
                DisplayEvent::Value(ValueDisplayEvent::Stay(v)) => *v == *value,
                DisplayEvent::Value(ValueDisplayEvent::ChangeEvent(v)) => *v == *value,
                _ => false,
            },
            ValueType::Vector(vector) => match self {
                DisplayEvent::Vector(VectorDisplayEvent::Stay(v)) => *v == *vector,
                DisplayEvent::Vector(VectorDisplayEvent::ChangeEvent(v)) => *v == *vector,
                _ => false,
            },
        }
    }
}

/// Convert a `Vector` value to its decimal value
/// Return None if the vector contains `x` or `z`
pub fn vector_to_base_10(vector: &Vector) -> Option<u64> {
    vector.iter().try_fold(0, |acc, value| match value {
        Value::V0 => Some(acc * 2),
        Value::V1 => Some(acc * 2 + 1),
        _ => None,
    })
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::Value(value) => write!(f, "{}", value),
            ValueType::Vector(vector) => match vector_to_base_10(vector) {
                Some(base_10) => write!(f, "{}", base_10),
                None => write!(f, "x"),
            },
        }
    }
}

pub struct Signal {
    // reference string in vcd file
    pub code: IdCode,
    pub name: String,
    pub events: Vec<(u64, ValueType)>,
    pub parent_module: Option<Weak<RefCell<Module>>>,
}

impl Signal {
    pub fn from_var(var: &Var) -> Signal {
        Signal {
            code: var.code,
            name: var.reference.clone(),
            events: vec![],
            parent_module: None,
        }
    }

    pub fn add_event(&mut self, timestamp: u64, value: ValueType) {
        self.events.push((timestamp, value));
    }
}

impl Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.events.len() == 0 {
            writeln!(f, "Signal: {}, code: {}", self.name, self.code)?;
        } else {
            writeln!(
                f,
                "Signal: {}, code: {}, events: {:?}",
                self.name, self.code, self.events
            )?;
        }
        Ok(())
    }
}

impl Signal {
    pub fn output_name(&self) -> String {
        format!("{}({})", self.name, self.code)
    }
    pub fn output_path(&self) -> String {
        let mut path =
            Module::get_path_str(&self.parent_module.clone().unwrap().upgrade().unwrap());
        if path.len() != 0 {
            path = path + ":"
        }
        format!("{}{}({})", path, self.name, self.code)
    }
    pub fn output_events(&self) -> String {
        format!("{:?}", self.events)
    }

    /// Output a string showing events in the given time range
    /// - `time_start` - the start time
    /// - `time_step` - the minimal time step
    /// - `arr_size` - the size of the final array
    pub fn events_str_in_range(&self, time_start: u64, time_step: u64, arr_size: usize) -> String {
        let time_end = time_start + time_step * arr_size as u64;
        self.events
            .iter()
            .fold(String::new(), |acc, (time, value)| {
                if time_start <= *time && *time <= time_end {
                    format!("{}({:?}), ", acc, (time, value))
                } else {
                    acc
                }
            })
    }

    /// Output a vector containing `DisplayEvents` in each time in the given time range
    /// - `time_start` - the start time
    /// - `time_step` - the minimal time step
    /// - `arr_size` - the size of the final array
    pub fn events_arr_in_range(
        &self,
        time_start: u64,
        time_step: u64,
        arr_size: usize,
    ) -> Vec<DisplayEvent> {
        let mut start_index = 0;
        let mut end_index = 0;

        while self.events[start_index].0 < time_start {
            start_index += 1;
            if start_index >= self.events.len() {
                break;
            }
        }

        let mut last_event =
            match self
                .events
                .get(if start_index == 0 { 0 } else { start_index - 1 })
            {
                Some(event) => match &event.1 {
                    ValueType::Value(value) => {
                        DisplayEvent::Value(ValueDisplayEvent::Stay(value.clone()))
                    }
                    ValueType::Vector(vector) => {
                        DisplayEvent::Vector(VectorDisplayEvent::Stay(vector.clone()))
                    }
                },
                None => DisplayEvent::Value(ValueDisplayEvent::Stay(Value::X)),
            };

        let mut event_arr = vec![last_event.clone(); arr_size];

        event_arr.iter_mut().enumerate().for_each(|(i, element)| {
            if start_index >= self.events.len() {
                *element = match &last_event {
                    DisplayEvent::Value(ValueDisplayEvent::ChangeEvent(value)) => {
                        DisplayEvent::Value(ValueDisplayEvent::Stay(value.clone()))
                    }
                    DisplayEvent::Vector(VectorDisplayEvent::ChangeEvent(vector)) => {
                        DisplayEvent::Vector(VectorDisplayEvent::Stay(vector.clone()))
                    }
                    _ => last_event.clone(),
                };
                return;
            }

            let start_time = time_start + (i as u64) * time_step;
            end_index = start_index;

            let end_time = start_time + time_step;

            if self.events[start_index].0 >= end_time {
                *element = match &last_event {
                    DisplayEvent::Value(ValueDisplayEvent::ChangeEvent(value)) => {
                        DisplayEvent::Value(ValueDisplayEvent::Stay(value.clone()))
                    }
                    DisplayEvent::Vector(VectorDisplayEvent::ChangeEvent(vector)) => {
                        DisplayEvent::Vector(VectorDisplayEvent::Stay(vector.clone()))
                    }
                    _ => last_event.clone(),
                };
                return;
            }

            while self.events[end_index].0 < end_time {
                end_index += 1;
                if end_index >= self.events.len() {
                    break;
                }
            }

            if end_index - start_index == 1 {
                let event_prev = last_event.clone();
                *element = match self.events[start_index].1.clone() {
                    ValueType::Value(value) => {
                        if event_prev == self.events[start_index].1 {
                            DisplayEvent::Value(ValueDisplayEvent::Stay(value))
                        } else {
                            DisplayEvent::Value(ValueDisplayEvent::ChangeEvent(value))
                        }
                    }

                    ValueType::Vector(vector) => {
                        if event_prev == self.events[start_index].1 {
                            DisplayEvent::Vector(VectorDisplayEvent::Stay(vector))
                        } else {
                            DisplayEvent::Vector(VectorDisplayEvent::ChangeEvent(vector))
                        }
                    }
                };
                last_event = element.clone();
            } else if end_index - start_index > 1 {
                *element = match self.events[start_index].1 {
                    ValueType::Value(_) => DisplayEvent::Value(ValueDisplayEvent::MultipleEvent),
                    ValueType::Vector(_) => DisplayEvent::Vector(VectorDisplayEvent::MultipleEvent),
                };
                last_event = element.clone();
            } else {
                panic!("No events in [start_time, end_time)")
            }

            start_index = end_index;
        });

        event_arr
    }
}
