use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr, Sub};

use php_parser_rs::lexer::byte_string::ByteString;
use php_parser_rs::lexer::token::Span;
use php_parser_rs::parser::ast::attributes::AttributeGroup;
use php_parser_rs::parser::ast::data_type::Type;
use php_parser_rs::parser::ast::functions::ReturnType;
use php_parser_rs::parser::ast::variables::SimpleVariable;
use php_parser_rs::parser::ast::Statement;

use crate::helpers::callable::php_value_matches_type;
use crate::helpers::helpers::get_string_from_bytes;

use super::php_object::PhpObject;

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
    String(ByteString),
    Array(HashMap<PhpValue, PhpValue>),
    Object(PhpObject),
    Callable(PhpCallable),
    Resource(Resource),
}

#[derive(Debug, Clone)]
pub struct PhpError {
    pub level: ErrorLevel,
    pub message: String,

    /// Note that in many parts of the program this field will be set to 0.
    /// This is because it is another part of the program that has the line
    /// where the error was generated and not the part that creates the structure.
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum ErrorLevel {
    Fatal,
    Warning,

    /// A Raw error should not be formatted with get_message().
    /// And it is for private use.
    Raw,
    /*	Notice,
    UserError,
    UserWarning,
    UserNotice, */
}

#[derive(Debug, Clone)]
pub enum Resource {}

#[derive(Debug, Clone)]
pub struct PhpCallable {
    pub attributes: Vec<AttributeGroup>,
    pub span: Span,
    pub return_by_reference: bool,
    pub name: ByteString,
    pub parameters: Vec<CallableArgument>,
    pub return_type: Option<ReturnType>,
    pub body: Vec<Statement>,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub struct CallableArgument {
    pub name: SimpleVariable,
    pub data_type: Option<Type>,
    pub default_value: Option<PhpValue>,
    pub pass_by_reference: bool,
    pub ellipsis: bool,
}

impl PhpValue {
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

    pub fn get_type_as_string(&self) -> String {
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
        let self_as_string = self.printable();
        let value_as_string = value.printable();

        if self_as_string.is_none() || value_as_string.is_none() {
            let error_message = format!(
                "Unsupported operation: {} . {}",
                self.get_type_as_string(),
                value.get_type_as_string()
            );

            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: error_message,
                line: 0,
            });
        }

        Ok(PhpValue::String(
            (self_as_string.unwrap() + &value_as_string.unwrap()).into(),
        ))
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
            PhpValue::String(s) => s.len() > 0,
            PhpValue::Array(a) => a.len() != 0,
            PhpValue::Object(_) => true,
            PhpValue::Callable(_) => true,
            PhpValue::Resource(_) => true,
        }
    }

    fn perform_arithmetic_operation<F>(
        &self,
        operation_sign: &str,
        rhs: PhpValue,
        operation: F,
    ) -> Result<PhpValue, PhpError>
    where
        F: Fn(f32, f32) -> f32,
    {
        let self_type = self.get_type_as_string();

        if self_type != INT && self_type != FLOAT {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Unsupported operation: {} {} {}",
                    self.get_type_as_string(),
                    operation_sign,
                    rhs.get_type_as_string()
                ),
                line: 0,
            });
        }

        let left_float = self.to_float();
        let right_float = rhs.to_float();

        if left_float.is_none() || right_float.is_none() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Unsupported operation: {} {} {}",
                    self.get_type_as_string(),
                    operation_sign,
                    rhs.get_type_as_string()
                ),
                line: 0,
            });
        }

        let left = left_float.unwrap();
        let right = right_float.unwrap();

        if self_type == INT {
            return Ok(PhpValue::Int(operation(left, right) as i32));
        } else {
            return Ok(PhpValue::Float(operation(left, right)));
        }
    }

    /// Returns the size of the value.
    fn get_size(&self) -> usize {
        match self {
            PhpValue::Int(i) => *i as usize,
            PhpValue::Float(f) => *f as usize,
            PhpValue::Bool(b) => b.to_string().len(),
            PhpValue::Null => 0,
            PhpValue::Callable(c) => c.name.bytes.len(),
            PhpValue::String(s) => s.len(),
            PhpValue::Array(a) => a.len(),
            _ => 0,
        }
    }

    pub fn is_iterable(&self) -> bool {
        match self {
            PhpValue::Array(_) => true,
            // TODO: PhpValue::Object(o) => o.is_instance_of("iterable"),
            _ => false,
        }
    }

    // Returns the value as a string.
    pub fn printable(&self) -> Option<String> {
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
            PhpValue::String(s) => Some(String::from_utf8_lossy(s).to_string()),
            PhpValue::Array(_) => None,
            PhpValue::Object(_) => None,
            PhpValue::Callable(c) => Some(get_string_from_bytes(&c.name.bytes)),
            PhpValue::Resource(_) => Some("Resource".to_string()),
        }
    }

    /*
     * Functions to convert to a data type.
     */

    pub fn to_int(&self) -> Option<i32> {
        match self {
            PhpValue::Int(i) => Some(*i),
            PhpValue::Float(f) => Some(*f as i32),
            PhpValue::String(s) => {
                let str_value = std::str::from_utf8(&s.bytes).unwrap();

                let int_value = str_value.parse();

                if int_value.is_err() {
                    return None;
                }

                return Some(int_value.unwrap());
            }
            _ => None,
        }
    }

    pub fn to_float(&self) -> Option<f32> {
        match self {
            PhpValue::Int(i) => Some(*i as f32),
            PhpValue::Float(f) => Some(*f),
            PhpValue::String(s) => {
                let str_value = std::str::from_utf8(&s.bytes).unwrap();

                let float_value = str_value.parse();

                if float_value.is_err() {
                    return None;
                }

                return Some(float_value.unwrap());
            }
            _ => None,
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self {
            PhpValue::Int(i) => Some(i.to_string()),
            PhpValue::Float(f) => Some(f.to_string()),
            PhpValue::String(s) => Some(String::from_utf8_lossy(s).to_string()),
            _ => None,
        }
    }
}

/*
 * Implementation of the arithmetic operators (and other traits)
 */

impl Add for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn add(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("+", rhs, |left, right| left + right)
    }
}

impl Sub for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("-", rhs, |left, right| left - right)
    }
}

impl Mul for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn mul(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("*", rhs, |left, right| left * right)
    }
}

impl Div for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn div(self, rhs: Self) -> Self::Output {
        let right_to_float = rhs.to_float();

        if right_to_float.is_none() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Unsupported operation: {} / {}",
                    self.get_type_as_string(),
                    rhs.get_type_as_string()
                ),
                line: 0,
            });
        }

        if right_to_float.unwrap() == 0.0 {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!("Division by zero"),
                line: 0,
            });
        }

        self.perform_arithmetic_operation("/", rhs, |left, right| left / right)
    }
}

impl Rem for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn rem(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("%", rhs, |left, right| left % right)
    }
}

impl BitAnd for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("&", rhs, |left, right| {
            (left as i32 & right as i32) as f32
        })
    }
}

impl BitOr for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("|", rhs, |left, right| {
            (left as i32 | right as i32) as f32
        })
    }
}

impl BitXor for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("^", rhs, |left, right| {
            (left as i32 ^ right as i32) as f32
        })
    }
}

impl Shl for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn shl(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("<<", rhs, |left, right| {
            let left_as_int = left as i32;
            let right_as_int = right as i32;

            (left_as_int << right_as_int) as f32
        })
    }
}

impl Shr for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn shr(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation(">>", rhs, |left, right| {
            let left_as_int = left as i32;
            let right_as_int = right as i32;

            (left_as_int >> right_as_int) as f32
        })
    }
}

impl Not for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn not(self) -> Self::Output {
        let self_clone = self.clone();

        match self_clone {
            PhpValue::Bool(b) => Ok(PhpValue::Bool(!b)),
            PhpValue::Int(i) => Ok(PhpValue::Bool(i == 0)),
            PhpValue::Float(f) => Ok(PhpValue::Bool(f == 0.0)),
            PhpValue::String(s) => Ok(PhpValue::Bool(s.len() == 0)),
            PhpValue::Null => Ok(PhpValue::Bool(true)),
            PhpValue::Array(a) => Ok(PhpValue::Bool(a.len() == 0)),
            _ => {
                let error_message =
                    format!("Unsupported operation: !{}", self.get_type_as_string());

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
        self.partial_cmp(other) == Some(Ordering::Equal)
	}

	fn ne(&self, other: &Self) -> bool {
		self.partial_cmp(other) != Some(Ordering::Equal)
	}
}

impl PartialOrd for PhpValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_size = self.get_size();
        let other_size = other.get_size();

        Some(self_size.cmp(&other_size))
    }
}

impl PhpError {
    pub fn get_message(self, input: &str) -> String {
        if let ErrorLevel::Raw = self.level {
            return self.message;
        }

        let level_error = match self.level {
            ErrorLevel::Fatal => "Fatal error",
            ErrorLevel::Warning => "Warning",
            _ => "",
        };

        format!(
            "PHP {}: {} in {} on line {}",
            level_error, self.message, input, self.line
        )
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

impl CallableArgument {
    /// Check that `arg` is valid for this argument.
    pub fn is_valid(&self, arg: &mut PhpValue, called_in_line: usize) -> Option<PhpError> {
        let self_has_type = &self.data_type;

        if self_has_type.is_some() {
            let self_type = self_has_type.as_ref().unwrap();

            return php_value_matches_type(self_type, arg, called_in_line);
        }

        None
    }
}
