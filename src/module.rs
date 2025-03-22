use core::fmt;
use std::fmt::Display;

use crate::signal::Signal;
use crate::signal::ValueType;

use vcd::{IdCode, Scope, ScopeItem, ScopeType, Value};

pub struct Module {
    pub(crate) name: String,
    pub(crate) depth: u8,
    pub(crate) signals: Vec<Signal>,
    pub(crate) submodules: Vec<Module>,
}

impl Module {
    pub fn from_scope(scope: &Scope, depth: u8) -> Module {
        assert!(scope.scope_type == ScopeType::Module);
        let mut signals = vec![];
        let mut sub_modules = vec![];

        for scope_type in &scope.items {
            match scope_type {
                ScopeItem::Var(var) => {
                    signals.push(Signal::from_var(var));
                }
                ScopeItem::Scope(sub_scope) => {
                    sub_modules.push(Module::from_scope(sub_scope, depth + 1))
                }
                _ => {}
            }
        }

        Module {
            name: scope.identifier.clone(),
            depth,
            signals,
            submodules: sub_modules,
        }
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Module: {}, depth: {}", self.name, self.depth)?;
        self.signals.iter().try_for_each(|x| {
            for _ in 0..self.depth {
                write!(f, "  ")?;
            }
            write!(f, "{x}")?;
            Ok(())
        })?;

        self.submodules.iter().try_for_each(|x| {
            for _ in 0..self.depth {
                write!(f, "  ")?;
            }
            write!(f, "{x}")?;
            Ok(())
        })?;
        Ok(())
    }
}

impl Module {
    pub fn add_event(&mut self, id: IdCode, timestamp: u64, value: ValueType) {
        self.signals
            .iter_mut()
            .filter(|x| x.code == id)
            .for_each(|x| x.add_event(timestamp, value.clone()));

        self.submodules
            .iter_mut()
            .for_each(|x| x.add_event(id, timestamp, value.clone()));
    }

    pub fn get_signals(&self) -> Vec<&Signal> {
        let mut signal_vec: Vec<&Signal> = self.signals.iter().map(|x| x).collect();

        self.submodules
            .iter()
            .for_each(|x| signal_vec.extend(x.get_signals()));

        signal_vec
    }

    pub fn max_time(&self) -> u64 {
        let mut max_time = 0;
        self.signals.iter().for_each(|x| {
            if let Some(time) = x.events.last() {
                if time.0 > max_time {
                    max_time = time.0;
                }
            }
        });

        self.submodules.iter().for_each(|x| {
            let time = x.max_time();
            if time > max_time {
                max_time = time;
            }
        });

        max_time
    }
}
