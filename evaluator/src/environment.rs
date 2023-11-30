use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use php_parser_rs::lexer::token::Span;

use crate::{
    helpers::get_string_from_bytes,
    php_value::{
        objects::{PhpClass, PhpObject},
        primitive_data_types::{ErrorLevel, PhpError, PhpValue},
    },
};

#[derive(Clone)]
pub struct Environment {
    vars: HashMap<Vec<u8>, Rc<RefCell<PhpValue>>>,

    /// All identifiers, such as functions or constants.
    identifiers: HashMap<Vec<u8>, PhpValue>,

    /// Determines whether modifications to the environment should be monitored, including the addition of new variables and functions.
    ///
    /// If set to `true`, any changes will be recorded in the `tracked_changes` field.
    /// Note: Deletion of a variable will not be included in the tracking.
    trace: bool,

    tracked_changes: TrackedChanges,

    objects: HashMap<Vec<u8>, PhpObject>,
}

#[derive(Clone)]
pub struct TrackedChanges {
    pub added_vars: Vec<Vec<u8>>,
    pub added_identifiers: Vec<Vec<u8>>,

    /// Contains a list of variables that have been modified.
    ///
    /// It is a map from the variable name to the value of the variable before the modification.
    pub modified_vars: HashMap<Vec<u8>, Rc<RefCell<PhpValue>>>,

    pub added_classes: Vec<Vec<u8>>,
}

impl TrackedChanges {
    pub fn new() -> TrackedChanges {
        TrackedChanges {
            added_vars: Vec::new(),
            added_identifiers: Vec::new(),
            modified_vars: HashMap::new(),
            added_classes: Vec::new(),
        }
    }
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            vars: HashMap::new(),
            identifiers: HashMap::new(),
            trace: false,
            tracked_changes: TrackedChanges::new(),
            objects: HashMap::new(),
        }
    }

    pub fn delete_var(&mut self, key: &[u8]) {
        self.vars.remove(key);
    }

    pub fn insert_var(&mut self, key: &[u8], value: &PhpValue) {
        if self.trace {
            match self.vars.entry(key.to_vec()) {
                Entry::Occupied(_) => {
                    let old_value = self.get_var_with_rc(key).unwrap().clone();

                    self.tracked_changes
                        .modified_vars
                        .insert(key.to_vec(), old_value);
                }
                Entry::Vacant(_) => {
                    self.tracked_changes.added_vars.push(key.to_vec());
                }
            }
        }

        self.vars
            .insert(key.to_vec(), Rc::new(RefCell::new(value.clone())));
    }

    pub fn insert_var_rc(&mut self, key: &[u8], value: Rc<RefCell<PhpValue>>) {
        if self.trace {
            match self.vars.entry(key.to_vec()) {
                Entry::Occupied(_) => {
                    self.tracked_changes
                        .modified_vars
                        .insert(key.to_vec(), value.clone());
                }
                Entry::Vacant(_) => {
                    self.tracked_changes.added_vars.push(key.to_vec());
                }
            }
        }

        self.vars.insert(key.to_vec(), value);
    }

    pub fn get_var(&self, key: &[u8]) -> Option<PhpValue> {
        let key = if key.is_empty() || key[0] != b'$' {
            let mut new_key: Vec<u8> = vec![b'$'];

            new_key.extend(key);

            new_key
        } else {
            key.to_vec()
        };

        let value = self.vars.get(&key);

        value.map(|value| value.borrow().clone())
    }

    pub fn var_exists(&self, key: &[u8]) -> bool {
        self.vars.contains_key(key)
    }

    pub fn get_var_with_rc(&self, key: &[u8]) -> Option<&Rc<RefCell<PhpValue>>> {
        self.vars.get(key)
    }

    pub fn get_ident(&self, key: &[u8]) -> Option<PhpValue> {
        self.identifiers.get(key).cloned()
    }

    pub fn start_trace(&mut self) {
        self.trace = true
    }

    /// Undoes all changes made to the environment based on the `tracked_changes` field.
    pub fn restore(&mut self) {
        self.trace = false;

        for key in self.tracked_changes.added_vars.iter() {
            self.vars.remove(key);
        }

        self.tracked_changes.added_vars.clear();

        for key in self.tracked_changes.added_identifiers.iter() {
            self.identifiers.remove(key);
        }

        // TODO: Not all identifiers should be deleted, only functions, identifiers such as constants should remain.
        self.tracked_changes.added_identifiers.clear();

        for (key, value) in self.tracked_changes.modified_vars.iter() {
            self.vars.insert(key.to_vec(), value.clone());
        }

        self.tracked_changes.modified_vars.clear();
    }

    pub fn new_ident(&mut self, ident: &[u8], value: PhpValue, span: Span) -> Option<PhpError> {
        match self.identifiers.entry(ident.to_vec()) {
            std::collections::hash_map::Entry::Occupied(entry) => Some(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot redeclare identifier {}",
                    get_string_from_bytes(entry.key())
                ),
                line: span.line,
            }),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(value);

                self.tracked_changes.added_identifiers.push(ident.to_vec());

                None
            }
        }
    }

    pub fn object_exists(&self, ident: &[u8]) -> bool {
        self.objects.contains_key(ident)
    }

    pub fn new_class(&mut self, name: &[u8], value: PhpObject, span: Span) -> Option<PhpError> {
        if self.object_exists(name) {
            Some(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot declare class {} because the name is already in use",
                    get_string_from_bytes(name)
                ),
                line: span.line,
            })
        } else {
            self.objects.insert(name.to_vec(), value);

            self.tracked_changes.added_classes.push(name.to_vec());

            None
        }
    }

    /// Retrieves the PHP class with the specified name if it exists.
    ///
    /// # Arguments
    ///
    /// * `ident`: A slice of bytes representing the name of the class to be retrieved.
    ///
    /// # Returns
    ///
    /// Returns an `Option` containing the requested `PhpClass` if the class exists; otherwise, returns `None`.
    /// Returns `None` if the conversion to `PhpClass` fails.
    ///
    /// # Panics
    ///
    /// Panics if the specified class name does not exist in the current environment.
    pub fn get_class(&self, ident: &[u8]) -> Option<PhpClass> {
        self.objects.get(ident).cloned().unwrap().into_class()
    }
}
