use core::{fmt, panic};
use std::{
    cell::RefCell,
    fmt::Display,
    rc::{Rc, Weak},
};

use crate::time::Time;

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

#[derive(Debug, Clone)]
pub enum SignalEvents {
    ValueEvents(Vec<(Time, Value)>),
    VectorEvents(Vec<(Time, Vector)>),
    Unknown,
}

pub struct Signal {
    // reference string in vcd file
    pub code: IdCode,
    pub name: String,
    pub events: SignalEvents,
    pub parent_module: Option<Weak<RefCell<Module>>>,
}

impl Signal {
    pub fn from_var(var: &Var) -> Signal {
        Signal {
            code: var.code,
            name: var.reference.clone(),
            events: SignalEvents::Unknown,
            parent_module: None,
        }
    }

    pub fn add_event(&mut self, timestamp: Time, value: ValueType) {
        if let SignalEvents::Unknown = self.events {
            self.events = match &value {
                ValueType::Value(_) => SignalEvents::ValueEvents(vec![]),
                ValueType::Vector(_) => SignalEvents::VectorEvents(vec![]),
            };
        }

        match self.events {
            SignalEvents::ValueEvents(ref mut events) => {
                if let ValueType::Value(v) = value {
                    events.push((timestamp, v));
                } else {
                    unreachable!("Mismatched event type for signal {}", self.name);
                }
            }
            SignalEvents::VectorEvents(ref mut events) => {
                if let ValueType::Vector(v) = value {
                    events.push((timestamp, v));
                } else {
                    unreachable!("Mismatched event type for signal {}", self.name);
                }
            }
            SignalEvents::Unknown => unreachable!("Signal events should not be Unknown here"),
        }
    }

    pub fn is_vector(&self) -> bool {
        matches!(self.events, SignalEvents::VectorEvents(_))
    }
}

impl Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Signal: {}, code: {}, events: {:?}",
            self.name, self.code, self.events
        )
    }
}

impl Signal {
    pub fn output_name(&self) -> String {
        format!("{}({})", self.name, self.code)
    }
    pub fn output_path(&self) -> String {
        let mut path =
            Module::get_path_str(&self.parent_module.clone().unwrap().upgrade().unwrap());
        if !path.is_empty() {
            path += ":"
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
    pub fn events_str_in_range(
        &self,
        time_start: Time,
        time_step: Time,
        arr_size: usize,
    ) -> String {
        let time_end = time_start + time_step * arr_size;
        match &self.events {
            SignalEvents::ValueEvents(events) => events
                .iter()
                .filter(|(time, _)| time_start <= *time && *time <= time_end)
                .map(|(time, value)| format!("({:?}, {:?})", time, value))
                .collect::<Vec<_>>()
                .join(", "),
            SignalEvents::VectorEvents(events) => events
                .iter()
                .filter(|(time, _)| time_start <= *time && *time <= time_end)
                .map(|(time, vector)| format!("({:?}, {:?})", time, vector))
                .collect::<Vec<_>>()
                .join(", "),
            SignalEvents::Unknown => panic!("Signal events are unknown"),
        }
    }

    /// Output a vector containing `DisplayEvents` in each time in the given time range
    /// - `time_start` - the start time
    /// - `time_step` - the minimal time step
    /// - `arr_size` - the size of the final array
    pub fn events_arr_in_range(
        &self,
        time_start: Time,
        time_step: Time,
        arr_size: usize,
    ) -> Vec<DisplayEvent> {
        let mut start_index: usize = 0;
        let mut end_index: usize = 0;

        match self.events {
            SignalEvents::Unknown => {
                panic!("Signal events are unknown")
            }
            SignalEvents::ValueEvents(ref events) => {
                while events[start_index].0 < time_start {
                    start_index += 1;
                    if start_index >= events.len() {
                        break;
                    }
                }

                let last_event = match events.get(start_index.saturating_sub(1)) {
                    Some(event) => ValueDisplayEvent::Stay(event.1),
                    None => ValueDisplayEvent::Stay(Value::X),
                };

                let last_event = Rc::new(RefCell::new(last_event));

                let mut event_arr = vec![last_event.borrow().clone(); arr_size];

                event_arr.iter_mut().enumerate().for_each(|(i, element)| {
                    if start_index >= events.len() {
                        *element = match &*last_event.borrow() {
                            ValueDisplayEvent::ChangeEvent(value) => {
                                ValueDisplayEvent::Stay(*value)
                            }
                            _ => last_event.borrow().clone(),
                        };
                        return;
                    }

                    let start_time = time_start + time_step * i;
                    end_index = start_index;

                    let end_time = start_time + time_step;

                    if events[start_index].0 >= end_time {
                        *element = match &*last_event.borrow() {
                            ValueDisplayEvent::ChangeEvent(value) => {
                                ValueDisplayEvent::Stay(*value)
                            }
                            _ => last_event.borrow().clone(),
                        };
                        return;
                    }

                    while events[end_index].0 < end_time {
                        end_index += 1;
                        if end_index >= events.len() {
                            break;
                        }
                    }

                    if end_index - start_index == 1 {
                        let event_prev = last_event.borrow().clone();
                        *element = match event_prev {
                            ValueDisplayEvent::ChangeEvent(v) | ValueDisplayEvent::Stay(v) => {
                                if v == events[start_index].1 {
                                    ValueDisplayEvent::Stay(events[start_index].1)
                                } else {
                                    ValueDisplayEvent::ChangeEvent(events[start_index].1)
                                }
                            }
                            ValueDisplayEvent::MultipleEvent => {
                                ValueDisplayEvent::ChangeEvent(events[start_index].1)
                            }
                        };
                        *last_event.borrow_mut() = element.clone();
                    } else if end_index - start_index > 1 {
                        *element = ValueDisplayEvent::MultipleEvent;
                        *last_event.borrow_mut() = element.clone();
                    } else {
                        panic!("No events in [start_time, end_time)")
                    }

                    start_index = end_index;
                });

                event_arr.into_iter().map(DisplayEvent::Value).collect()
            }
            SignalEvents::VectorEvents(ref events) => {
                while events[start_index].0 < time_start {
                    start_index += 1;
                    if start_index >= events.len() {
                        break;
                    }
                }

                let last_event = match events.get(start_index.saturating_sub(1)) {
                    Some(event) => VectorDisplayEvent::Stay(event.1.clone()),
                    None => {
                        let len = events.first().unwrap().1.len();
                        let mut x_arr: Vec<Value> = vec![];
                        x_arr.resize(len, Value::X);

                        let vector_x = Vector::from(x_arr);
                        VectorDisplayEvent::Stay(vector_x)
                    }
                };

                let last_event = Rc::new(RefCell::new(last_event));

                let mut event_arr = vec![last_event.borrow().clone(); arr_size];

                event_arr.iter_mut().enumerate().for_each(|(i, element)| {
                    if start_index >= events.len() {
                        *element = match &*last_event.borrow() {
                            VectorDisplayEvent::ChangeEvent(vector) => {
                                VectorDisplayEvent::Stay(vector.clone())
                            }
                            _ => last_event.borrow().clone(),
                        };
                        return;
                    }

                    let start_time = time_start + time_step * i;
                    end_index = start_index;

                    let end_time = start_time + time_step;

                    if events[start_index].0 >= end_time {
                        *element = match &*last_event.borrow() {
                            VectorDisplayEvent::ChangeEvent(vector) => {
                                VectorDisplayEvent::Stay(vector.clone())
                            }
                            _ => last_event.borrow().clone(),
                        };
                        return;
                    }

                    while events[end_index].0 < end_time {
                        end_index += 1;
                        if end_index >= events.len() {
                            break;
                        }
                    }

                    if end_index - start_index == 1 {
                        let event_prev = last_event.borrow().clone();
                        *element = match event_prev {
                            VectorDisplayEvent::ChangeEvent(v) | VectorDisplayEvent::Stay(v) => {
                                if v == events[start_index].1 {
                                    VectorDisplayEvent::Stay(events[start_index].1.clone())
                                } else {
                                    VectorDisplayEvent::ChangeEvent(events[start_index].1.clone())
                                }
                            }
                            VectorDisplayEvent::MultipleEvent => {
                                VectorDisplayEvent::ChangeEvent(events[start_index].1.clone())
                            }
                        };
                        *last_event.borrow_mut() = element.clone();
                    } else if end_index - start_index > 1 {
                        *element = VectorDisplayEvent::MultipleEvent;
                        *last_event.borrow_mut() = element.clone();
                    } else {
                        panic!("No events in [start_time, end_time)")
                    }

                    start_index = end_index;
                });
                event_arr.into_iter().map(DisplayEvent::Vector).collect()
            }
        }
    }
}
