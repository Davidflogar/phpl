use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr, Sub};

use php_parser_rs::parser::ast::Statement;

pub const NULL: &str = "null";
pub const BOOL: &str = "bool";
pub const INT: &str = "int";
pub const FLOAT: &str = "float";
pub const STRING: &str = "string";
pub const ARRAY: &str = "array";
pub const OBJECT: &str = "object";
pub const CALLABLE: &str = "callable";
pub const RESOURCE: &str = "resource";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PhpValue {
    Null,
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Array(Vec<PhpValue>),
    Object(PhpObject),
    Callable(PhpCallable),
    Resource(Resource),
}

#[derive(Debug, Clone)]
pub struct PhpError {
    pub level: ErrorLevel,
    pub message: String,
	pub line: usize,
}

#[derive(Debug, Clone)]
pub enum ErrorLevel {
    Fatal,
    Warning,
	ParseError,

	/// A Raw error should not be formatted with get_message()
	Raw,
    /*	Notice,
    UserError,
    UserWarning,
    UserNotice, */
}

#[derive(Debug, Clone)]
pub enum Resource {}

#[derive(Debug, Clone)]
pub struct PhpObject {
    pub name: String,
    pub properties: Vec<PhpValue>,
    pub methods: Vec<PhpCallable>,
    pub parent: Option<Box<PhpObject>>,
}

#[derive(Debug, Clone)]
pub struct PhpCallable {
    pub name: String,
    pub parameters: Vec<PhpValue>,
    pub body: Vec<Statement>,
}

impl PhpValue {
    pub fn to_string(&self) -> Option<String> {
        match self {
            PhpValue::Null => Some("NULL".to_string()),
            PhpValue::Bool(b) => {
                if *b {
                    Some("1".to_string())
                } else {
                    Some("".to_string())
                }
            }
            PhpValue::Int(i) => Some(i.to_string()),
            PhpValue::Float(f) => Some(f.to_string()),
            PhpValue::String(s) => Some(s.to_string()),
            PhpValue::Array(_) => None,
            PhpValue::Object(_) => None,
            PhpValue::Callable(_) => None,
            PhpValue::Resource(_) => Some("Resource".to_string()),
        }
    }

    /// Performs a power operation on two values.
    pub fn pow(self, value: PhpValue) -> Result<PhpValue, PhpError> {
        match (self, value) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i.pow(j as u32))),
            (PhpValue::Float(f), PhpValue::Float(g)) => Ok(PhpValue::Float(f.powf(g))),
            (PhpValue::Int(i), PhpValue::Float(f)) => {
                let f = f as f32;
                let i = i as f32;

                Ok(PhpValue::Float(i.powf(f)))
            }
            (PhpValue::Float(f), PhpValue::Int(i)) => {
                let f = f as f32;
                let i = i as f32;

                Ok(PhpValue::Float(f.powf(i)))
            }
            _ => {
                let error_message = "Unsupported operation".to_string();

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }

    pub fn get_type(self) -> String {
        match self {
            PhpValue::Null => NULL.to_string(),
            PhpValue::Bool(_) => BOOL.to_string(),
            PhpValue::Int(_) => INT.to_string(),
            PhpValue::Float(_) => FLOAT.to_string(),
            PhpValue::String(_) => STRING.to_string(),
            PhpValue::Array(_) => ARRAY.to_string(),
            PhpValue::Object(_) => OBJECT.to_string(),
            PhpValue::Callable(_) => CALLABLE.to_string(),
            PhpValue::Resource(_) => RESOURCE.to_string(),
        }
    }

    /// Concatenates two values.
    pub fn concat(self, value: PhpValue) -> Result<PhpValue, PhpError> {
        let self_as_string = self.to_string();
        let value_as_string = value.to_string();

        if self_as_string.is_none() || value_as_string.is_none() {
            let error_message = format!(
                "Unsupported operation: {} + {}",
                self.get_type(),
                value.get_type()
            );

            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: error_message,
				line: 0,
            });
        }

        Ok(PhpValue::String(self_as_string.unwrap() + &value_as_string.unwrap()))
    }

    pub fn is_null(&self) -> bool {
        match self {
            PhpValue::Null => true,
            _ => false,
        }
    }

    /// Checks if the value is "true" in PHP terms.
    pub fn is_true(self) -> bool {
        match self {
            PhpValue::Null => false,
            PhpValue::Bool(b) => b,
            PhpValue::Int(i) => i != 0,
            PhpValue::Float(f) => f != 0.0,
            PhpValue::String(s) => s != "",
            PhpValue::Array(a) => a.len() != 0,
            PhpValue::Object(_) => true,
            PhpValue::Callable(_) => true,
            PhpValue::Resource(_) => true,
        }
    }
}

/*
 * Implementation of the arithmetic operators (and other traits)
 */

impl Add for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn add(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i + j)),
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Float(i as f32 + f)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Float(f + i as f32)),
            (PhpValue::Float(f), PhpValue::Float(g)) => Ok(PhpValue::Float(f + g)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} + {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Sub for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn sub(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i - j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => Ok(PhpValue::Float(f - g)),
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Float(i as f32 - f)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Float(f - i as f32)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} - {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Mul for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn mul(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i * j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => Ok(PhpValue::Float(f * g)),
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Float(i as f32 * f)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Float(f * i as f32)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} * {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Div for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn div(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i / j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => Ok(PhpValue::Float(f / g)),
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Float(i as f32 / f)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Float(f / i as f32)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} / {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Rem for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn rem(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i % j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => Ok(PhpValue::Float(f % g)),
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Float(i as f32 % f)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Float(f % i as f32)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} % {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl BitAnd for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitand(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i & j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => {
                let f_as_int = f as i32;
                let g_as_int = g as i32;

                Ok(PhpValue::Int(f_as_int & g_as_int))
            }
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Int(i & f as i32)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Int(f as i32 & i)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} & {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl BitOr for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitor(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i | j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => {
                let f_as_int = f as i32;
                let g_as_int = g as i32;

                Ok(PhpValue::Int(f_as_int | g_as_int))
            }
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Int(i & f as i32)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Int(f as i32 & i)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} | {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl BitXor for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i ^ j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => {
                let f_as_int = f as i32;
                let g_as_int = g as i32;

                Ok(PhpValue::Int(f_as_int ^ g_as_int))
            }
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Int(i & f as i32)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Int(f as i32 & i)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} ^ {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Shl for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn shl(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i << j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => {
                let f_as_int = f as i32;
                let g_as_int = g as i32;

                Ok(PhpValue::Int(f_as_int << g_as_int))
            }
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Int(i & f as i32)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Int(f as i32 & i)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} << {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Shr for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn shr(self, rhs: Self) -> Self::Output {
        let self_clone = self.clone();
        let rhs_clone = rhs.clone();

        match (self_clone, rhs_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => Ok(PhpValue::Int(i >> j)),
            (PhpValue::Float(f), PhpValue::Float(g)) => {
                let f_as_int = f as i32;
                let g_as_int = g as i32;

                Ok(PhpValue::Int(f_as_int >> g_as_int))
            }
            (PhpValue::Int(i), PhpValue::Float(f)) => Ok(PhpValue::Int(i & f as i32)),
            (PhpValue::Float(f), PhpValue::Int(i)) => Ok(PhpValue::Int(f as i32 & i)),
            _ => {
                let error_message = format!(
                    "Unsupported operation: {} >> {}",
                    self.get_type(),
                    rhs.get_type()
                );

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl Not for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn not(self) -> Self::Output {
        let self_clone = self.clone();

        match self_clone {
            PhpValue::Bool(b) => Ok(PhpValue::Bool(!b)),
            _ => {
                let error_message = format!("Unsupported operation: !{}", self.get_type());

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: error_message,
					line: 0,
                })
            }
        }
    }
}

impl PartialEq for PhpValue {
    fn eq(&self, other: &Self) -> bool {
        let self_clone = self.clone();
        let other_clone = other.clone();

        match (self_clone, other_clone) {
            (PhpValue::Null, PhpValue::Null) => true,
            (PhpValue::Bool(b), PhpValue::Bool(c)) => b == c,
            (PhpValue::Int(i), PhpValue::Int(j)) => i == j,
            (PhpValue::Float(i), PhpValue::Float(j)) => i == j,
            (PhpValue::Float(i), PhpValue::Int(j)) => i == j as f32,
            (PhpValue::Int(i), PhpValue::Float(j)) => i as f32 == j,
            (PhpValue::String(i), PhpValue::String(j)) => i == j,
            (PhpValue::String(i), PhpValue::Int(j)) => i == j.to_string(),
            (PhpValue::Int(i), PhpValue::String(j)) => i.to_string() == j,
            (PhpValue::Array(i), PhpValue::Array(j)) => i == j,
            // (PhpValue::Object(i), PhpValue::Object(j)) => i == j, TODO
            // (PhpValue::Callable(i), PhpValue::Callable(j)) => i == j, TODO
            // (PhpValue::Resource(i), PhpValue::Resource(j)) => i == j, TODO
            _ => false,
        }
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl PartialOrd for PhpValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_clone = self.clone();
        let other_clone = other.clone();

        // TODO: implement automathic type conversion

        match (self_clone, other_clone) {
            (PhpValue::Int(i), PhpValue::Int(j)) => i.partial_cmp(&j),
            (PhpValue::Float(i), PhpValue::Float(j)) => i.partial_cmp(&j),
            (PhpValue::Float(i), PhpValue::Int(j)) => i.partial_cmp(&(j as f32)),
            (PhpValue::Int(i), PhpValue::Float(j)) => (i as f32).partial_cmp(&j),
            (PhpValue::String(i), PhpValue::String(j)) => i.partial_cmp(&j),
            (PhpValue::String(i), PhpValue::Int(j)) => i.partial_cmp(&j.to_string()),
            (PhpValue::Int(i), PhpValue::String(j)) => i.to_string().partial_cmp(&j),
            _ => None,
        }
    }
}

impl PhpObject {
    pub fn is_instance_of(self, object: PhpValue) -> Result<bool, PhpError> {
        if let PhpValue::Object(object) = object {
            if object.name == self.name {
                return Ok(true);
            }

            if self.parent.is_some() && self.parent.unwrap().name == object.name {
                return Ok(true);
            }

            Ok(false)
        } else {
            Err(PhpError {
                level: ErrorLevel::Fatal,
                message: "Right side of instanceof must be an object".to_string(),
				line: 0,
            })
        }
    }
}

impl PhpError {
	pub fn get_message(self, input: &str) -> String {
		if let ErrorLevel::Raw = self.level {
			return self.message;
		}

		let level_error = match self.level {
			ErrorLevel::Fatal => "Fatal error".to_string(),
			ErrorLevel::Warning => "Warning".to_string(),
			ErrorLevel::ParseError => "Parse error".to_string(),
			_ => "".to_string(),
		};

		format!("PHP {}: {} in {} on line {}", level_error, self.message, input, self.line)
	}
}

impl From<String> for PhpError {
	fn from(message: String) -> Self {
		PhpError {
			level: ErrorLevel::Fatal,
			message,
			line: 0,
		}
	}
}
