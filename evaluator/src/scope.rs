use std::{cell::RefCell, collections::HashMap, rc::Rc};

use php_parser_rs::lexer::token::Span;

use crate::{
    errors::cannot_redeclare_object,
    helpers::get_string_from_bytes,
    php_value::{
        error::{ErrorLevel, PhpError},
        objects::{PhpObject, PhpObjectType},
        primitive_data_types::{PhpIdentifier, PhpValue},
    },
};

#[derive(Clone)]
pub struct Scope {
    vars: HashMap<Vec<u8>, Rc<RefCell<PhpValue>>>,

    /// Identifiers such as functions or constants.
    identifiers: HashMap<Vec<u8>, PhpIdentifier>,

    objects: HashMap<Vec<u8>, PhpObject>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            vars: HashMap::new(),
            identifiers: HashMap::new(),
            objects: HashMap::new(),
        }
    }

    pub fn delete_var(&mut self, key: &[u8]) {
        self.vars.remove(key);
    }

    /// Sets the value of a variable. If the variable does not exist, it is created.
    /// But if the variable exists and is a reference, it is updated.
    pub fn set_var_value(&mut self, key: &[u8], value: PhpValue) {
        if let Some(var) = self.vars.get(key) {
            let mut var_ref = var.borrow_mut();

            match &mut *var_ref {
                PhpValue::Reference(ref_value) => {
                    *ref_value.borrow_mut() = value;
                }
                _ => {
                    *var_ref = value;
                }
            }

            return;
        }

        self.vars.insert(key.to_vec(), Rc::new(RefCell::new(value)));
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

    /// Returns a reference to the value of the variable. If the variable does not exist, it is created.
    pub fn new_ref(&mut self, to: &[u8]) -> Rc<RefCell<PhpValue>> {
        if !self.var_exists(to) {
            self.set_var_value(to, PhpValue::Null);
        }

        let reference = self.vars.get(to).unwrap();

        Rc::clone(reference)
    }

    pub fn get_ident(&self, key: &[u8]) -> Option<&PhpIdentifier> {
        self.identifiers.get(key)
    }

    pub fn new_ident(
        &mut self,
        ident: &[u8],
        value: PhpIdentifier,
        span: Span,
    ) -> Result<(), PhpError> {
        match self.identifiers.entry(ident.to_vec()) {
            std::collections::hash_map::Entry::Occupied(entry) => Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot redeclare identifier {}, there is already a {} using this name",
                    get_string_from_bytes(entry.key()),
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
        self.objects.contains_key(ident)
    }

    pub fn new_object(
        &mut self,
        value: PhpObject,
        object_type: PhpObjectType,
    ) -> Result<(), PhpError> {
        if self.object_exists(value.get_name_as_bytes()) {
            Err(cannot_redeclare_object(
                value.get_name_as_bytes(),
                value.get_name_span().line,
                object_type,
            ))
        } else {
            self.objects
                .insert(value.get_name_as_bytes().to_vec(), value);

            Ok(())
        }
    }

    pub fn get_object_cloned(&self, ident: &[u8]) -> Option<PhpObject> {
        self.objects.get(ident).cloned()
    }

    pub fn get_object_by_ref(&self, ident: &[u8]) -> Option<&PhpObject> {
        self.objects.get(ident)
    }
}
