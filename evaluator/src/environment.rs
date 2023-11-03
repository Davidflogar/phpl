use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::php_value::PhpValue;

#[derive(Clone)]
pub struct Environment {
    vars: HashMap<Vec<u8>, Rc<RefCell<PhpValue>>>,
    identifiers: HashMap<Vec<u8>, PhpValue>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            vars: HashMap::new(),
            identifiers: HashMap::new(),
        }
    }

    pub fn delete_var(&mut self, key: &[u8]) {
        self.vars.remove(key);
    }

    pub fn set_var(&mut self, key: &[u8], value: &PhpValue) {
        self.vars.insert(key.to_vec(), Rc::new(RefCell::new(value.clone())));
    }

    pub fn set_var_rc(&mut self, key: &[u8], value: Rc<RefCell<PhpValue>>) {
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

        match value {
            Some(value) => Some(value.borrow().clone()),
            None => None,
        }
    }

    pub fn var_exists(&self, key: &[u8]) -> bool {
        self.vars.contains_key(key)
    }

    pub fn get_var_with_rc(&self, key: &[u8]) -> Option<&Rc<RefCell<PhpValue>>> {
        self.vars.get(key)
    }

    pub fn get_identifier(&self, key: &[u8]) -> Option<PhpValue> {
        self.identifiers.get(key).cloned()
    }

    /// Merges differences from another environment, adding missing values.
    pub fn get_and_set_diff(&mut self, other_env: Environment) {
        for (key, value) in other_env.vars {
            self.vars.entry(key).or_insert(value);
        }

        for (key, value) in other_env.identifiers {
            self.identifiers.insert(key, value);
        }
    }

    pub fn identifier_entry(
        &mut self,
        key: Vec<u8>,
    ) -> std::collections::hash_map::Entry<'_, Vec<u8>, PhpValue> {
        self.identifiers.entry(key)
    }
}
