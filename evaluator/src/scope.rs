use std::{cell::RefCell, collections::HashMap, rc::Rc};

use php_parser_rs::lexer::token::Span;

use crate::{
    errors::cannot_redeclare_object,
    helpers::{get_string_from_bytes, string_as_number},
    php_data_types::{
        error::{ErrorLevel, PhpError},
        objects::PhpObject,
        primitive_data_types::{PhpIdentifier, PhpValue},
    },
};

#[derive(Clone)]
pub struct Scope {
    vars: HashMap<u64, PhpValue>,

    /// Identifiers such as functions or constants.
    identifiers: HashMap<u64, PhpIdentifier>,

    objects: HashMap<u64, PhpObject>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            vars: HashMap::new(),
            identifiers: HashMap::new(),
            objects: HashMap::new(),
        }
    }

    /// Sets the value of a variable. If the variable does not exist, it is created.
    fn set_var_value(&mut self, key: u64, new_value: PhpValue) {
        if let Some(var) = self.vars.get_mut(&key) {
            match new_value {
                PhpValue::Reference(reference_to) => {
                    *var = PhpValue::Reference(reference_to);
                }
                PhpValue::Owned(value) => match var {
                    PhpValue::Owned(_) => {
                        *var = PhpValue::Owned(value);
                    }
                    PhpValue::Reference(reference_to) => {
                        *reference_to.borrow_mut() = value;
                    }
                },
            }
        } else {
            self.vars.insert(key, new_value);
        }
    }

    pub fn add_var_value(&mut self, key: Vec<u8>, new_value: PhpValue) {
        let key = string_as_number(&key);

        self.set_var_value(key, new_value);
    }

    pub fn add_var_value_with_raw_key(&mut self, key: u64, new_value: PhpValue) {
        self.set_var_value(key, new_value);
    }

    pub fn get_var(&self, key: &[u8]) -> Option<&PhpValue> {
        let key = if key.is_empty() || key[0] != b'$' {
            let mut new_key = vec![b'$'];

            new_key.extend(key);

            new_key
        } else {
            key.to_vec()
        };

        self.vars.get(&string_as_number(&key))
    }

    pub fn delete_var(&mut self, key: &[u8]) -> Option<PhpValue> {
        let key = if key.is_empty() || key[0] != b'$' {
            let mut new_key = vec![b'$'];

            new_key.extend(key);

            new_key
        } else {
            key.to_vec()
        };

        self.vars.remove(&string_as_number(&key))
    }

    pub fn var_exists(&self, key: &[u8]) -> bool {
        self.vars.contains_key(&string_as_number(key))
    }

    /// Returns a reference to the value of the variable. If the variable does not exist, it is created.
    pub fn new_ref(&mut self, to: Vec<u8>) -> PhpValue {
        let to = string_as_number(&to);

        self.vars.entry(to).or_insert_with(PhpValue::new_null);

        let reference = self.vars.remove(&to).unwrap();

        match reference {
            PhpValue::Owned(value) => {
                // convert the value to a reference
                let value_to_reference = Rc::new(RefCell::new(value));

                let return_value = PhpValue::Reference(Rc::clone(&value_to_reference));

                // insert the value back into the scope
                self.vars
                    .insert(to, PhpValue::Reference(value_to_reference));

                return_value
            }
            PhpValue::Reference(reference) => {
                let return_value = PhpValue::Reference(Rc::clone(&reference));

                // insert the reference back into the scope
                self.vars.insert(to, PhpValue::Reference(reference));

                return_value
            }
        }
    }

    pub fn get_ident(&self, key: &[u8]) -> Option<&PhpIdentifier> {
        self.identifiers.get(&string_as_number(key))
    }

    pub fn new_ident(
        &mut self,
        ident: &[u8],
        value: PhpIdentifier,
        span: Span,
    ) -> Result<(), PhpError> {
        match self.identifiers.entry(string_as_number(ident)) {
            std::collections::hash_map::Entry::Occupied(entry) => Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot redeclare identifier {}, there is already a {} using this name",
                    get_string_from_bytes(ident),
                    if entry.get().is_function() {
                        "function"
                    } else {
                        "constant"
                    }
                ),
                line: span.line,
            }),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(value);

                Ok(())
            }
        }
    }

    pub fn object_exists(&self, ident: &[u8]) -> bool {
        self.objects.contains_key(&string_as_number(ident))
    }

    pub fn new_object(&mut self, value: PhpObject) -> Result<(), PhpError> {
        if self.object_exists(value.get_name_as_bytes()) {
            Err(cannot_redeclare_object(
                value.get_name_as_bytes(),
                value.get_name_span().line,
            ))
        } else {
            self.objects
                .insert(string_as_number(value.get_name_as_bytes()), value);

            Ok(())
        }
    }

    pub fn get_object_cloned(&self, ident: &[u8]) -> Option<PhpObject> {
        self.objects.get(&string_as_number(ident)).cloned()
    }

    pub fn get_object_by_ref(&self, ident: &[u8]) -> Option<&PhpObject> {
        self.objects.get(&string_as_number(ident))
    }
}
