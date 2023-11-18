use php_parser_rs::parser::ast::data_type::Type;

use crate::{php_value::{PhpValue, PhpError}, errors};

/// Checks if a PHP value matches a type.
///
/// It also tries to convert php_value to the type if it's not already the same type.
pub fn php_value_matches_type(
    r#type: &Type,
    php_value: &mut PhpValue,
    called_in_line: usize,
) -> Option<PhpError> {
    match r#type {
        Type::Named(_, _) => todo!(),
        Type::Nullable(_, r#type) => {
            if let PhpValue::Null = php_value {
                return None;
            }

            php_value_matches_type(r#type, php_value, called_in_line)
        }
        Type::Union(types) => {
            let matches_any = types
                .iter()
                .any(|ty| php_value_matches_type(ty, php_value, called_in_line).is_none());

            if !matches_any {
                return Some(errors::expected_type_but_got(
                    &types
                        .iter()
                        .map(|ty| ty.to_string())
                        .collect::<Vec<_>>()
                        .join("|"),
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            None
        }
        Type::Intersection(types) => {
            for ty in types {
                if let Some(_) = php_value_matches_type(ty, php_value, called_in_line) {
                    return Some(errors::expected_type_but_got(
                        &types
                            .iter()
                            .map(|ty| ty.to_string())
                            .collect::<Vec<_>>()
                            .join("&"),
                        php_value.get_type_as_string(),
                        called_in_line,
                    ));
                }
            }

            None
        }
        Type::Void(_) => unreachable!(),
        Type::Null(_) => {
            if !matches!(php_value, PhpValue::Null) {
                return Some(errors::expected_type_but_got(
                    "null",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            None
        }
        Type::True(_) => {
            let PhpValue::Bool(b) = *php_value else {
                return Some(errors::expected_type_but_got(
                    "true",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            };

            if !b {
                return Some(errors::expected_type_but_got(
                    "true",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            *php_value = PhpValue::Bool(true);

            None
        }
        Type::False(_) => {
            let PhpValue::Bool(b) = php_value else {
                return Some(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            };

            if *b {
                return Some(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            *php_value = PhpValue::Bool(false);

            None
        }
        Type::Never(_) => unreachable!(),
        Type::Float(_) => {
            let to_float = php_value.to_float();

            if to_float.is_none() {
                return Some(errors::expected_type_but_got(
                    "float",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            *php_value = PhpValue::Float(to_float.unwrap());

            None
        }
        Type::Boolean(_) => {
            if !matches!(php_value, PhpValue::Bool(_)) {
                return Some(errors::expected_type_but_got(
                    "boolean",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            None
        }
        Type::Integer(_) => {
            let is_int = php_value.to_int();

            if is_int.is_none() {
                return Some(errors::expected_type_but_got(
                    "int",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            *php_value = PhpValue::Int(is_int.unwrap());

            None
        }
        Type::String(_) => {
            let is_string = php_value.to_string();

            if is_string.is_none() {
                return Some(errors::expected_type_but_got(
                    "string",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            *php_value = PhpValue::String(is_string.unwrap().into());

            None
        }
        Type::Array(_) => {
            if !matches!(php_value, PhpValue::Array(_)) {
                return Some(errors::expected_type_but_got(
                    "array",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            None
        }
        Type::Object(_) => {
            if !matches!(php_value, PhpValue::Object(_)) {
                return Some(errors::expected_type_but_got(
                    "object",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            None
        }
        Type::Mixed(_) => None,
        Type::Callable(_) => {
            if !matches!(php_value, PhpValue::Callable(_)) {
                return Some(errors::expected_type_but_got(
                    "callable",
                    php_value.get_type_as_string(),
                    called_in_line,
                ));
            }

            None
        }
        Type::Iterable(_) => todo!(),
        Type::StaticReference(_) => unreachable!(),
        Type::SelfReference(_) => todo!(),
        Type::ParentReference(_) => todo!(),
    }
}
