use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr, Sub};
use std::rc::Rc;

use php_parser_rs::lexer::byte_string::ByteString;
use php_parser_rs::lexer::token::Span;
use php_parser_rs::parser::ast::arguments::Argument;
use php_parser_rs::parser::ast::attributes::AttributeGroup;
use php_parser_rs::parser::ast::functions::ReturnType;
use php_parser_rs::parser::ast::{Expression, ReferenceExpression, Statement};

use crate::errors::{expected_type_but_got, only_arrays_and_traversables_can_be_unpacked};
use crate::evaluator::Evaluator;
use crate::expressions::reference;
use crate::helpers::php_value_matches_argument_type;

use super::argument_type::PhpArgumentType;
use super::error::{ErrorLevel, PhpError};
use super::macros::impl_validate_argument_for_struct;
use super::objects::PhpObject;

impl_validate_argument_for_struct!(PhpFunctionArgument);

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
    String(Vec<u8>),
    Array(HashMap<PhpValue, PhpValue>),
    Object(PhpObject),
    Callable(PhpCallable),
    Resource(Resource),
    Reference(Rc<RefCell<PhpValue>>),
}

#[derive(Debug, Clone)]
pub enum Resource {}

#[derive(Debug, Clone)]
pub struct PhpCallable {
    pub attributes: Vec<AttributeGroup>,
    pub return_by_reference: bool,
    pub parameters: Vec<PhpFunctionArgument>,
    pub return_type: Option<ReturnType>,
    pub body: Vec<Statement>,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub struct PhpFunctionArgument {
    pub name: ByteString,
    pub data_type: Option<PhpArgumentType>,
    pub default_value: Option<PhpValue>,
    pub is_variadic: bool,
    pub pass_by_reference: bool,
}

impl PartialEq for PhpFunctionArgument {
    fn eq(&self, other: &Self) -> bool {
        if self.name != other.name {
            return false;
        }

        if self.data_type.is_some() && other.data_type.is_some() {
            let self_data_type = self.data_type.as_ref().unwrap();
            let other_data_type = other.data_type.as_ref().unwrap();

            if !(self_data_type == other_data_type) {
                return false;
            }
        } else if self.data_type.is_none() != other.data_type.is_some() {
            return false;
        }

        if self.default_value.is_some() && other.default_value.is_some() {
            let self_default_value = self.default_value.as_ref().unwrap();
            let other_default_value = other.default_value.as_ref().unwrap();

            if !(self_default_value == other_default_value) {
                return false;
            }
        }

        if self.is_variadic != other.is_variadic {
            return false;
        }

        true
    }
}

impl PhpValue {
    pub fn is_null(&self) -> bool {
        match self {
            PhpValue::Null => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_null(),
            _ => false,
        }
    }

    pub fn is_bool(&self) -> bool {
        match self {
            PhpValue::Bool(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_bool(),
            _ => false,
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            PhpValue::Int(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_int(),
            _ => false,
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            PhpValue::Float(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_float(),
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            PhpValue::String(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_string(),
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            PhpValue::Array(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_array(),
            _ => false,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            PhpValue::Object(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_object(),
            _ => false,
        }
    }

    pub fn is_callable(&self) -> bool {
        match self {
            PhpValue::Callable(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().is_callable(),
            _ => false,
        }
    }

    pub fn pow(self, value: PhpValue) -> Result<PhpValue, PhpError> {
        self.perform_arithmetic_operation("**", value, |a, b| a.powf(b))
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
            PhpValue::Reference(ref_value) => ref_value.borrow().get_type_as_string(),
        }
    }

    pub fn concat(self, value: PhpValue) -> Result<PhpValue, PhpError> {
        let self_as_string = self.as_string();
        let value_as_string = value.as_string();

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

    /// Checks if the value is "true" in PHP terms.
    pub fn true_in_php(&self) -> bool {
        match self {
            PhpValue::Null => false,
            PhpValue::Bool(b) => *b,
            PhpValue::Int(i) => *i != 0,
            PhpValue::Float(f) => *f != 0.0,
            PhpValue::String(s) => !s.is_empty(),
            PhpValue::Array(a) => !a.is_empty(),
            PhpValue::Object(_) => true,
            PhpValue::Callable(_) => true,
            PhpValue::Resource(_) => true,
            PhpValue::Reference(ref_value) => ref_value.borrow().true_in_php(),
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

        let left_float = self.as_float();
        let right_float = rhs.as_float();

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
            Ok(PhpValue::Int(operation(left, right) as i32))
        } else {
            Ok(PhpValue::Float(operation(left, right)))
        }
    }

    /// Returns the size of the value.
    fn get_size(&self) -> usize {
        match self {
            PhpValue::Int(i) => *i as usize,
            PhpValue::Float(f) => *f as usize,
            PhpValue::Bool(b) => (*b).into(),
            PhpValue::Null => 0,
            PhpValue::Callable(_) => 1,
            PhpValue::String(s) => s.len(),
            PhpValue::Array(a) => a.len(),
            PhpValue::Reference(ref_value) => ref_value.borrow().get_size(),
            _ => 0,
        }
    }

    pub fn is_iterable(&self) -> bool {
        match self {
            PhpValue::Array(_) => true,
            // TODO: PhpValue::Object(o) => o.is_instance_of("iterable"),
            PhpValue::Reference(ref_value) => ref_value.borrow().is_iterable(),
            _ => false,
        }
    }

    // Returns the value as a string.
    pub fn printable(&self) -> Option<String> {
        match self {
            PhpValue::Null => Some("".to_string()),
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
            PhpValue::Callable(_) => None,
            PhpValue::Resource(_) => Some("Resource".to_string()),
            PhpValue::Reference(value) => value.borrow().printable(),
        }
    }

    /*
     * Functions to convert to a data type.
     */

    pub fn as_float(&self) -> Option<f32> {
        match self {
            PhpValue::Int(i) => Some(*i as f32),
            PhpValue::Float(f) => Some(*f),
            PhpValue::String(s) => {
                let str_value = std::str::from_utf8(s).unwrap();

                let float_value = str_value.parse();

                if float_value.is_err() {
                    return None;
                }

                Some(float_value.unwrap())
            }
            PhpValue::Reference(ref_value) => ref_value.borrow().as_float(),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            PhpValue::Int(i) => Some(i.to_string()),
            PhpValue::Float(f) => Some(f.to_string()),
            PhpValue::String(s) => Some(String::from_utf8_lossy(s).to_string()),
            PhpValue::Reference(ref_value) => ref_value.borrow().as_string(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PhpValue::Bool(b) => Some(*b),
            PhpValue::Reference(ref_value) => ref_value.borrow().as_bool(),
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
        let right_to_float = rhs.as_float();

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
                message: "Division by zero".to_string(),
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
            PhpValue::String(s) => Ok(PhpValue::Bool(s.is_empty())),
            PhpValue::Null => Ok(PhpValue::Bool(true)),
            PhpValue::Array(a) => Ok(PhpValue::Bool(a.is_empty())),
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
}

impl PartialOrd for PhpValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_size = self.get_size();
        let other_size = other.get_size();

        Some(self_size.cmp(&other_size))
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

#[derive(Clone)]
#[allow(dead_code)]
pub enum PhpIdentifier {
    Constant(PhpValue),
    Function(PhpCallable),
}

impl PhpIdentifier {
    pub fn is_function(&self) -> bool {
        matches!(self, PhpIdentifier::Function(_))
    }
}
