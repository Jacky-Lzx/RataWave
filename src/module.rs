use core::fmt;
use std::fmt::Display;

use crate::signal::Signal;

use vcd::{Scope, ScopeItem, ScopeType};

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
