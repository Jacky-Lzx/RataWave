use core::fmt;
use std::{
    cell::RefCell,
    fmt::Display,
    rc::{Rc, Weak},
};

use cli_log::debug;
use vcd::{IdCode, Scope, ScopeItem, ScopeType};

use super::signal::{Signal, ValueType};

/// A module struct representing modules in the VCD file.
/// A root module is created to contain the top-level signals.
pub struct Module {
    pub(crate) name: String,
    pub(crate) depth: u8,
    pub(crate) signals: Vec<Rc<RefCell<Signal>>>,
    pub(crate) submodules: Vec<Rc<RefCell<Module>>>,
    pub(crate) parent: Option<Weak<RefCell<Module>>>,
}

impl Module {
    /// Build a module from the scope
    /// The parent of the module is set to None
    pub fn from_scope(scope: &Scope, depth: u8) -> Rc<RefCell<Module>> {
        assert!(scope.scope_type == ScopeType::Module);
        let mut signals = vec![];
        let mut sub_modules = vec![];

        for scope_type in &scope.items {
            match scope_type {
                ScopeItem::Var(var) => {
                    signals.push(Rc::new(RefCell::new(Signal::from_var(var))));
                }
                ScopeItem::Scope(sub_scope) => {
                    sub_modules.push(Module::from_scope(sub_scope, depth + 1))
                }
                _ => {}
            }
        }

        let module = Rc::new(RefCell::new(Module {
            name: scope.identifier.clone(),
            depth,
            signals,
            submodules: sub_modules,
            parent: None,
        }));

        module
            .borrow_mut()
            .submodules
            .iter()
            .for_each(|x| x.borrow_mut().parent = Some(Rc::downgrade(&module)));

        module
            .borrow_mut()
            .signals
            .iter()
            .for_each(|x| x.borrow_mut().parent_module = Some(Rc::downgrade(&module)));

        module
    }
}
impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self)?;
        Ok(())
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Module: {}, depth: {}", self.name, self.depth)?;
        self.signals.iter().try_for_each(|x| {
            for _ in 0..self.depth {
                write!(f, "  ")?;
            }
            write!(f, "{}", x.borrow())?;
            Ok(())
        })?;

        self.submodules.iter().try_for_each(|x| {
            for _ in 0..self.depth {
                write!(f, "  ")?;
            }
            write!(f, "{}", x.borrow())?;
            Ok(())
        })?;
        Ok(())
    }
}

impl Module {
    pub fn add_event(&mut self, id: IdCode, timestamp: u64, value: ValueType) {
        self.signals
            .iter_mut()
            .filter(|x| x.borrow_mut().code == id)
            .for_each(|x| x.borrow_mut().add_event(timestamp, value.clone()));

        self.submodules
            .iter_mut()
            .for_each(|x| x.borrow_mut().add_event(id, timestamp, value.clone()));
    }

    pub fn get_signals(&self) -> Vec<Rc<RefCell<Signal>>> {
        let mut signal_vec: Vec<Rc<RefCell<Signal>>> =
            self.signals.iter().map(|x| Rc::clone(x)).collect();

        self.submodules
            .iter()
            .for_each(|x| signal_vec.extend(x.borrow().get_signals()));

        signal_vec
    }

    pub fn max_time(&self) -> u64 {
        let mut max_time = 0;
        self.signals.iter().for_each(|x| {
            if let Some(time) = x.borrow().events.last() {
                if time.0 > max_time {
                    max_time = time.0;
                }
            }
        });

        self.submodules.iter().for_each(|x| {
            let time = x.borrow().max_time();
            if time > max_time {
                max_time = time;
            }
        });

        max_time
    }

    pub fn get_path_str(s: &Rc<RefCell<Module>>) -> String {
        // Get the path of the module from the root
        let mut path = vec![];
        let mut node = Rc::clone(s);

        while let Some(parent_weak) = {
            let borrowed_node = node.borrow();
            borrowed_node.parent.clone()
        } {
            if let Some(parent_rc) = parent_weak.upgrade() {
                if parent_rc.borrow().parent.is_none() {
                    break; // Stop if we reach the root module
                }
                path.insert(0, parent_rc.borrow().name.clone());
                node = parent_rc;
            } else {
                break; // Handle the case where the parent has been dropped
            }
        }
        // while let Some(parent) = node.borrow().parent.clone() {
        //     path.insert(0, parent.upgrade().unwrap().borrow().name.clone());
        //     node = Rc::clone(&parent.upgrade().unwrap());
        // }

        path.join("->")
    }
}
