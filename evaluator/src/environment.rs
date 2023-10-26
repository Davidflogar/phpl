use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::php_type::PhpValue;

#[derive(Clone)]
pub struct Environment {
    pub vars: HashMap<String, Rc<RefCell<PhpValue>>>,
}

impl Environment {
    pub fn delete(&mut self, key: &str) {
        self.vars.remove(key);
    }

    pub fn set(&mut self, key: &str, value: PhpValue) {
        self.vars
            .insert(key.to_string(), Rc::new(RefCell::new(value)));
    }

    pub fn set_rc(&mut self, key: &str, value: Rc<RefCell<PhpValue>>) {
        self.vars.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<PhpValue> {
        let key = if !key.starts_with('$') {
            format!("${}", key)
        } else {
            key.to_string()
        };

        let value = self.vars.get(&key);

        match value {
            Some(value) => Some(value.borrow().clone()),
            None => None,
        }
    }

    pub fn exists(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    pub fn get_var_with_rc(&self, key: &str) -> Option<&Rc<RefCell<PhpValue>>> {
        self.vars.get(key)
    }
}
