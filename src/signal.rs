use core::fmt;
use std::fmt::Display;

use vcd::{IdCode, Value, Var, Vector};

/// Type of the signal
/// - `Value`: the signal has only one bit
/// - `Vector`: the signal has multiple bits
#[derive(Debug, Clone)]
pub enum ValueType {
    Value(Value),
    Vector(Vector),
}

pub struct EdgeRepresentation {
    pub first_line: &'static str,
    pub second_line: &'static str,
}

/// For one-bit signal, the waveform is represented in two lines.
/// `RISING_EDGE` consists of the characters for the first and second line, respectively.
/// It looks as follows
/// ```text
///       ┌
///       ┘
/// ```
pub const RISING_EDGE: EdgeRepresentation = EdgeRepresentation {
    first_line: "┌",
    second_line: "┘",
};

/// For one-bit signal, the waveform is represented in two lines.
/// `FALLING_EDGE` consists of the characters for the first and second line, respectively.
/// It looks as follows
/// ```text
///       ┐
///       └
/// ```
pub const FALLING_EDGE: EdgeRepresentation = EdgeRepresentation {
    first_line: "┐",
    second_line: "└",
};

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
}

impl Signal {
    pub fn from_var(var: &Var) -> Signal {
        Signal {
            code: var.code,
            name: var.reference.clone(),
            events: vec![],
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

    /// Output a vector containing values in each time in the given time range
    /// - `time_start` - the start time
    /// - `time_step` - the minimal time step
    /// - `arr_size` - the size of the final array
    pub fn events_arr_in_range(
        &self,
        time_start: u64,
        time_step: u64,
        arr_size: usize,
    ) -> Vec<&ValueType> {
        let mut event_arr = vec![&self.events.first().unwrap().1; arr_size];
        let mut event_index = 0;

        for (i, element) in event_arr.iter_mut().enumerate() {
            let time = time_start + (i as u64) * time_step;
            if self.events[event_index].0 > time {
                assert!(self.events[event_index - 1].0 <= time);
                *element = &self.events[event_index - 1].1;
            } else {
                *element = &self.events[event_index].1;
                event_index += 1;
                if event_index >= self.events.len() {
                    break;
                }
            }
        }

        event_arr
    }
}
