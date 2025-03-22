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

/// For one-bit signal, the waveform is represented in two lines.
/// `RISING_EDGE` consists of the characters for the first and second line, respectively.
/// It looks as follows
/// ```text
///       ┌
///       ┘
/// ```
pub const RISING_EDGE: (&str, &str) = ("┌", "┘");
/// For one-bit signal, the waveform is represented in two lines.
/// `FALLING_EDGE` consists of the characters for the first and second line, respectively.
/// It looks as follows
/// ```text
///       ┐
///       └
/// ```
pub const FALLING_EDGE: (&str, &str) = ("┐", "└");

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
    pub fn output_events_in_range(&self, time_range: (u64, u64)) -> String {
        self.events
            .iter()
            .fold(String::new(), |acc, (time, value)| {
                if time_range.0 <= *time && *time <= time_range.1 {
                    format!("{}({:?}), ", acc, (time, value))
                } else {
                    acc
                }
            })
        // format!("{:?}", self.events)
    }
    pub fn event_arr_in_range(&self, time_range: (u64, u64), time_scale: u64) -> Vec<&ValueType> {
        // self.events
        //     .iter()
        //     .fold(String::new(), |acc, (time, value)| {
        //         if time_range.0 <= *time && *time <= time_range.1 {
        //             format!("{}({:?}), ", acc, (time, value))
        //         } else {
        //             acc
        //         }
        //     })
        let arr_size = (time_range.1 - time_range.0).div_ceil(time_scale) as usize;
        let mut event_arr = vec![&self.events.first().unwrap().1; arr_size];
        let mut last_index = 0;
        let mut last_value = &self.events.first().unwrap().1;

        for event in self.events.iter() {
            let event_time = event.0;
            if event_time >= time_range.1 {
                break;
            }

            if event_time >= time_range.0 {
                let value = &event.1;
                let index = (event_time - time_range.0).div_ceil(time_scale) as usize;
                // if time_range.0 <= event_time && event_time <= time_range.1 {
                //     event_arr[index] = &value;
                // }
                for i in last_index..index {
                    event_arr[i] = &last_value;
                }

                last_value = value;
                last_index = index;
            }
        }

        event_arr
    }
}
