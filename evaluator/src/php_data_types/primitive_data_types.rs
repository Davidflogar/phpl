use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Add, BitAnd, BitOr, BitXor, Deref, Div, Mul, Not, Rem, Shl, Shr, Sub};
use std::rc::Rc;

use php_parser_rs::lexer::byte_string::ByteString;
use php_parser_rs::lexer::token::Span;
use php_parser_rs::parser::ast::arguments::Argument;
use php_parser_rs::parser::ast::attributes::AttributeGroup;
use php_parser_rs::parser::ast::functions::ReturnType;
use php_parser_rs::parser::ast::{Expression, ReferenceExpression, Statement};
use smallvec::SmallVec;

use crate::errors::{expected_type_but_got, only_arrays_and_traversables_can_be_unpacked};
use crate::evaluator::Evaluator;
use crate::expressions::reference;
use crate::helpers::{get_string_from_bytes, php_value_matches_argument_type};

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
//pub const ARRAY: &str = "array";
pub const OBJECT: &str = "object";
//pub const CALLABLE: &str = "callable";
//pub const RESOURCE: &str = "resource";

const MAX_STRING_SIZE: usize = 30;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum PhpDataType {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(SmallVec<[u8; MAX_STRING_SIZE]>),
    Array,
    Object(PhpObject),
    Callable(PhpCallable),
    Resource,
}

#[derive(Debug)]
pub enum PhpValue {
    Owned(PhpDataType),
    Reference(Rc<RefCell<PhpDataType>>),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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

/// A simple struct to work with borrowed values inside a PhpValue.
pub enum BorrowedValue<'a, T>
where
    T: ?Sized,
{
    /// A reference to the owned value.
    Owned(&'a T),

    /// A reference to the borrowed value.
    Reference(Ref<'a, T>),
}

impl<T> Deref for BorrowedValue<'_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            BorrowedValue::Owned(value) => value,
            BorrowedValue::Reference(value) => value,
        }
    }
}

impl PhpValue {
    fn new(value: PhpDataType) -> Self {
        PhpValue::Owned(value)
    }

    pub fn new_null() -> Self {
        PhpValue::new(PhpDataType::Null)
    }

    pub fn new_bool(value: bool) -> Self {
        PhpValue::new(PhpDataType::Bool(value))
    }

    pub fn new_int(value: i64) -> Self {
        PhpValue::new(PhpDataType::Int(value))
    }

    pub fn new_float(value: f64) -> Self {
        PhpValue::new(PhpDataType::Float(value))
    }

    pub fn new_string(value: Vec<u8>) -> Self {
        let vec: SmallVec<[_; MAX_STRING_SIZE]> = SmallVec::from_vec(value);

        PhpValue::new(PhpDataType::String(vec))
    }

    pub fn new_object(value: PhpObject) -> Self {
        PhpValue::new(PhpDataType::Object(value))
    }

    pub fn is_null(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Null) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::Null)
            }
        }
    }

    pub fn is_bool(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Bool(_)) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::Bool(_))
            }
        }
    }

    pub fn is_int(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Int(_)) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::Int(_))
            }
        }
    }

    pub fn is_float(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Float(_)) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::Float(_))
            }
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::String(_)) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::String(_))
            }
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Float(_)) => true,
            PhpValue::Owned(PhpDataType::Int(_)) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::Float(_)) || matches!(&*value, &PhpDataType::Int(_))
            }
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Object(_)) => true,
            PhpValue::Owned(_) => false,
            PhpValue::Reference(ref value) => {
                let value = value.borrow();

                matches!(&*value, &PhpDataType::Object(_))
            }
        }
    }

    pub fn get_type_as_string(&self) -> String {
        let get_type_as_string = |self_borrowed: &PhpDataType| match self_borrowed {
            PhpDataType::Null => NULL.to_string(),
            PhpDataType::Bool(_) => BOOL.to_string(),
            PhpDataType::Int(_) => INT.to_string(),
            PhpDataType::Float(_) => FLOAT.to_string(),
            PhpDataType::String(_) => STRING.to_string(),
            PhpDataType::Object(_) => OBJECT.to_string(),
            _ => todo!(),
        };

        match self {
            PhpValue::Owned(value) => get_type_as_string(value),
            PhpValue::Reference(value) => get_type_as_string(&value.borrow()),
        }
    }

    /// Checks if the value is "true" in PHP terms.
    pub fn true_in_php(&self) -> bool {
        let is_true = |self_borrowed: &PhpDataType| match self_borrowed {
            PhpDataType::Null => false,
            PhpDataType::Bool(b) => *b,
            PhpDataType::Int(i) => *i != 0,
            PhpDataType::Float(f) => *f != 0.0,
            PhpDataType::String(string) => string.is_empty(),
            _ => todo!(),
        };

        match self {
            PhpValue::Owned(value) => is_true(value),
            PhpValue::Reference(value) => is_true(&value.borrow()),
        }
    }

    /// Returns the "size" of the value.
    fn get_size(&self) -> usize {
        let get_size = |self_borrowed: &PhpDataType| match self_borrowed {
            PhpDataType::Null => 0,
            PhpDataType::Bool(b) => *b as usize,
            PhpDataType::Int(i) => *i as usize,
            PhpDataType::Float(f) => *f as usize,
            PhpDataType::String(string) => string.len(),
            _ => todo!(),
        };

        match self {
            PhpValue::Owned(value) => get_size(value),
            PhpValue::Reference(value) => get_size(&value.borrow()),
        }
    }

    pub fn is_iterable(&self) -> bool {
        todo!()
    }

    /// Returns the value as a string if it is printable.
    pub fn printable(&self) -> Option<String> {
        let printable = |self_borrowed: &PhpDataType| match self_borrowed {
            PhpDataType::Null => Some(NULL.to_string()),
            PhpDataType::Bool(b) => Some(b.to_string()),
            PhpDataType::Int(i) => Some(i.to_string()),
            PhpDataType::Float(f) => Some(f.to_string()),
            PhpDataType::String(string) => Some(get_string_from_bytes(string.as_slice())),
            _ => todo!(),
        };

        match self {
            PhpValue::Owned(value) => printable(value),
            PhpValue::Reference(value) => printable(&value.borrow()),
        }
    }

    pub fn pow(self, value: PhpValue) -> Result<PhpValue, PhpError> {
        self.perform_arithmetic_operation("**", value, |a, b| a.powf(b))
    }

    pub fn concat(self, other: PhpValue) -> Result<PhpValue, PhpError> {
        if !self.is_string() || !other.is_string() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Unsupported operation: {} . {}",
                    self.get_type_as_string(),
                    other.get_type_as_string()
                ),
                line: 0,
            });
        }

        let self_as_string = self.as_string();
        let other_as_string = other.as_string();

        let mut result = Vec::new();
        result.extend(self_as_string.as_ref());
        result.extend(other_as_string.as_ref());

        Ok(PhpValue::new_string(result))
    }

    fn perform_arithmetic_operation<F>(
        &self,
        operation_sign: &str,
        rhs: PhpValue,
        operation: F,
    ) -> Result<PhpValue, PhpError>
    where
        F: Fn(f64, f64) -> f64,
    {
        if !self.is_number() || !rhs.is_number() {
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

        let left = self.as_float();
        let right = rhs.as_float();

        if self.is_int() && rhs.is_int() {
            Ok(PhpValue::new_int(operation(left, right) as i64))
        } else {
            Ok(PhpValue::new_float(operation(left, right)))
        }
    }

    /*
     * The following functions are used to convert the value to a specific type.
     * If the value is not of the specified type, the program will panic, so the data type must be checked before calling these functions.
     */

    pub fn as_string(&self) -> BorrowedValue<[u8]> {
        match self {
            PhpValue::Owned(PhpDataType::String(string)) => BorrowedValue::Owned(string.as_slice()),
            PhpValue::Reference(value) => {
                let value = value.borrow();

                BorrowedValue::Reference(Ref::map(value, |v| {
                    let PhpDataType::String(string) = v else {
                        unimplemented!();
                    };

                    string.as_slice()
                }))
            }
            _ => unimplemented!(),
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            PhpValue::Owned(PhpDataType::Float(f)) => *f,
            PhpValue::Owned(PhpDataType::Int(i)) => *i as f64,
            PhpValue::Reference(value) => {
                let value = value.borrow();

                let PhpDataType::Float(f) = &*value else {
                    unimplemented!();
                };

                *f
            }
            _ => unimplemented!(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            PhpValue::Owned(PhpDataType::Bool(b)) => *b,
            PhpValue::Reference(value) => {
                let value = value.borrow();

                let PhpDataType::Bool(b) = &*value else {
                    unimplemented!();
                };

                *b
            }
            _ => unimplemented!(),
        }
    }

    pub fn as_object(&self) -> BorrowedValue<PhpObject> {
        match self {
            PhpValue::Owned(PhpDataType::Object(o)) => BorrowedValue::Owned(o),
            PhpValue::Reference(reference) => {
                let value = reference.borrow();

                BorrowedValue::Reference(Ref::map(value, |v| {
                    let PhpDataType::Object(o) = v else {
                        unimplemented!();
                    };

                    o
                }))
            }
            _ => unimplemented!(),
        }
    }

    pub fn into_string(self) -> SmallVec<[u8; 30]> {
        match self {
            PhpValue::Owned(PhpDataType::String(string)) => string,
            PhpValue::Reference(value) => match Rc::try_unwrap(value) {
                Ok(value) => {
                    let PhpDataType::String(string) = value.into_inner() else {
                        unimplemented!();
                    };

                    string
                }
                Err(_) => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }
}

impl Clone for PhpValue {
    fn clone(&self) -> Self {
        match self {
            PhpValue::Owned(value) => PhpValue::new(value.clone()),
            PhpValue::Reference(value) => PhpValue::new(value.borrow().clone()),
        }
    }
}

impl PartialEq for PhpFunctionArgument {
    fn eq(&self, other: &Self) -> bool {
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

            if self_default_value != other_default_value {
                return false;
            }
        }

        if self.is_variadic != other.is_variadic {
            return false;
        }

        true
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
        if !rhs.is_number() {
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

        let right_as_float = rhs.as_float();

        if right_as_float == 0.0 {
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
            (left as i64 & right as i64) as f64
        })
    }
}

impl BitOr for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("|", rhs, |left, right| {
            (left as i64 | right as i64) as f64
        })
    }
}

impl BitXor for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn bitxor(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("^", rhs, |left, right| {
            (left as i64 ^ right as i64) as f64
        })
    }
}

impl Shl for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn shl(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation("<<", rhs, |left, right| {
            let left_as_int = left as i64;
            let right_as_int = right as i64;

            (left_as_int << right_as_int) as f64
        })
    }
}

impl Shr for PhpValue {
    type Output = Result<PhpValue, PhpError>;

    fn shr(self, rhs: Self) -> Self::Output {
        self.perform_arithmetic_operation(">>", rhs, |left, right| {
            let left_as_int = left as i64;
            let right_as_int = right as i64;

            (left_as_int >> right_as_int) as f64
        })
    }
}

impl Not for PhpValue {
    type Output = PhpValue;

    fn not(self) -> Self::Output {
        let not = |self_borrowed: &PhpDataType| match self_borrowed {
            PhpDataType::Null => PhpValue::new_bool(true),
            PhpDataType::Bool(b) => PhpValue::new_bool(!b),
            PhpDataType::Int(i) => PhpValue::new_bool(*i == 0),
            PhpDataType::Float(f) => PhpValue::new_bool(*f == 0.0),
            PhpDataType::String(string) => PhpValue::new_bool(string.is_empty()),
            _ => todo!(),
        };

        match self {
            PhpValue::Owned(value) => not(&value),
            PhpValue::Reference(value) => not(&value.borrow()),
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
    Constant(PhpDataType),
    Function(PhpCallable),
}

impl PhpIdentifier {
    pub fn is_function(&self) -> bool {
        matches!(self, PhpIdentifier::Function(_))
    }

    pub fn as_php_value_cloned(&self) -> PhpValue {
        match self {
            PhpIdentifier::Constant(value) => PhpValue::new(value.clone()),
            PhpIdentifier::Function(callable) => {
                PhpValue::new(PhpDataType::Callable(callable.clone()))
            }
        }
    }
}
