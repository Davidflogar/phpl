use std::fmt::Display;

use php_parser_rs::parser::ast::data_type::Type;

use crate::{helpers::get_string_from_bytes, scope::Scope};

use super::{
    error::{ErrorLevel, PhpError},
    objects::PhpObject,
};

/// An enum that represents all data types that are valid to use as parameter in php.
#[derive(Debug, Clone)]
pub enum PhpArgumentType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Array,
    Object,
    Callable,
    Union(Vec<PhpArgumentType>),
    Intersection(Vec<PhpArgumentType>),
    Mixed,
    Nullable(Box<PhpArgumentType>),
    Iterable,
    StaticReference,
    SelfReference,
    ParentReference,
    True,
    False,
    /// A named type, such as a class or trait.
    Named(PhpObject),
}

impl PhpArgumentType {
    /// Converts a `Type` to a `PhpArgumentType`.
    ///
    /// The `scope` is only used with named types, as they can be a class or a trait.
    pub fn from_type(value: &Type, scope: &Scope) -> Result<Self, PhpError> {
        match value {
            Type::Named(span, name) => {
                let Some(object) = scope.get_object(name) else {
					return Err(PhpError {
						level: ErrorLevel::Fatal,
						message: format!("Undefined type {}",
						get_string_from_bytes(name)),
						line: span.line
					})
				};

                Ok(PhpArgumentType::Named(object))
            }
            Type::Nullable(_, r#type) => Ok(PhpArgumentType::Nullable(Box::new(
                PhpArgumentType::from_type(r#type, scope)?,
            ))),
            Type::Union(union) => {
                let mut vec_types = vec![];

                for r#type in union {
                    vec_types.push(PhpArgumentType::from_type(r#type, scope)?);
                }

                Ok(PhpArgumentType::Union(vec_types))
            }
            Type::Intersection(intersection) => {
                let mut vec_types = vec![];

                for r#type in intersection {
                    vec_types.push(PhpArgumentType::from_type(r#type, scope)?);
                }

                Ok(PhpArgumentType::Intersection(vec_types))
            }
            Type::Void(_) => unreachable!(),
            Type::Null(_) => Ok(PhpArgumentType::Null),
            Type::True(_) => Ok(PhpArgumentType::True),
            Type::False(_) => Ok(PhpArgumentType::False),
            Type::Never(_) => unreachable!(),
            Type::Float(_) => Ok(PhpArgumentType::Float),
            Type::Boolean(_) => Ok(PhpArgumentType::Bool),
            Type::Integer(_) => Ok(PhpArgumentType::Int),
            Type::String(_) => Ok(PhpArgumentType::String),
            Type::Array(_) => Ok(PhpArgumentType::Array),
            Type::Object(_) => Ok(PhpArgumentType::Object),
            Type::Mixed(_) => Ok(PhpArgumentType::Mixed),
            Type::Callable(_) => Ok(PhpArgumentType::Callable),
            Type::Iterable(_) => Ok(PhpArgumentType::Iterable),
            Type::StaticReference(_) => Ok(PhpArgumentType::StaticReference),
            Type::SelfReference(_) => Ok(PhpArgumentType::SelfReference),
            Type::ParentReference(_) => Ok(PhpArgumentType::ParentReference),
        }
    }
}

impl Display for PhpArgumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhpArgumentType::Named(inner) => write!(f, "{}", inner.get_name()),
            PhpArgumentType::Nullable(inner) => write!(f, "?{}", inner),
            PhpArgumentType::Union(inner) => write!(
                f,
                "{}",
                inner
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join("|")
            ),
            PhpArgumentType::Intersection(inner) => write!(
                f,
                "{}",
                inner
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<String>>()
                    .join("&")
            ),
            PhpArgumentType::Null => write!(f, "null"),
            PhpArgumentType::True => write!(f, "true"),
            PhpArgumentType::False => write!(f, "false"),
            PhpArgumentType::Float => write!(f, "float"),
            PhpArgumentType::Bool => write!(f, "bool"),
            PhpArgumentType::Int => write!(f, "int"),
            PhpArgumentType::String => write!(f, "string"),
            PhpArgumentType::Array => write!(f, "array"),
            PhpArgumentType::Object => write!(f, "object"),
            PhpArgumentType::Mixed => write!(f, "mixed"),
            PhpArgumentType::Callable => write!(f, "callable"),
            PhpArgumentType::Iterable => write!(f, "iterable"),
            PhpArgumentType::StaticReference => write!(f, "static"),
            PhpArgumentType::SelfReference => write!(f, "self"),
            PhpArgumentType::ParentReference => write!(f, "parent"),
        }
    }
}

impl PartialEq for PhpArgumentType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PhpArgumentType::Null, PhpArgumentType::Null) => true,
            (PhpArgumentType::Bool, PhpArgumentType::Bool) => true,
            (PhpArgumentType::Int, PhpArgumentType::Int) => true,
            (PhpArgumentType::Float, PhpArgumentType::Float) => true,
            (PhpArgumentType::String, PhpArgumentType::String) => true,
            (PhpArgumentType::Array, PhpArgumentType::Array) => true,
            (PhpArgumentType::Object, PhpArgumentType::Object) => true,
            (PhpArgumentType::Callable, PhpArgumentType::Callable) => true,
            (PhpArgumentType::Union(a), PhpArgumentType::Union(b)) => a == b,
            (PhpArgumentType::Intersection(a), PhpArgumentType::Intersection(b)) => a == b,
            (PhpArgumentType::Mixed, PhpArgumentType::Mixed) => true,
            (PhpArgumentType::Nullable(a), PhpArgumentType::Nullable(b)) => a == b,
            (PhpArgumentType::Iterable, PhpArgumentType::Iterable) => true,
            (PhpArgumentType::StaticReference, PhpArgumentType::StaticReference) => true,
            (PhpArgumentType::SelfReference, PhpArgumentType::SelfReference) => true,
            (PhpArgumentType::ParentReference, PhpArgumentType::ParentReference) => true,
            (PhpArgumentType::True, PhpArgumentType::True) => true,
            (PhpArgumentType::False, PhpArgumentType::False) => true,
            (PhpArgumentType::Named(a), PhpArgumentType::Named(b)) => a.instance_of(b),
            _ => false,
        }
    }
}
