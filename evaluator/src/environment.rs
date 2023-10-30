use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::php_value::PhpValue;

#[derive(Clone)]
pub struct Environment {
    vars: HashMap<String, Rc<RefCell<PhpValue>>>,
	identifiers: HashMap<String, PhpValue>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            vars: HashMap::new(),
			identifiers: HashMap::new(),
        }
    }

    pub fn delete_var(&mut self, key: &str) {
        self.vars.remove(key);
    }

    pub fn set(&mut self, key: &str, value: PhpValue) {
        self.vars
            .insert(key.to_string(), Rc::new(RefCell::new(value)));
    }

    pub fn set_var_rc(&mut self, key: &str, value: Rc<RefCell<PhpValue>>) {
        self.vars.insert(key.to_string(), value);
    }

    pub fn get_var(&self, key: &str) -> Option<PhpValue> {
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

    pub fn var_exists(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    pub fn get_var_with_rc(&self, key: &str) -> Option<&Rc<RefCell<PhpValue>>> {
        self.vars.get(key)
    }

	pub fn get_identifier(&self, key: &str) -> Option<PhpValue> {
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
}
