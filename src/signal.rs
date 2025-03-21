use core::fmt;
use std::fmt::Display;

use vcd::{IdCode, Value, Var, Vector};

#[derive(Debug, Clone)]
pub enum ValueType {
    Value(Value),
    Vector(Vector),
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
