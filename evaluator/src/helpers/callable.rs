use php_parser_rs::parser::ast::{data_type::Type, functions::FunctionParameterList};

use crate::{
    errors,
    php_value::types::{CallableArgument, PhpError, PhpValue, ErrorLevel}, evaluator::Evaluator,
};

use super::get_string_from_bytes;

/// Checks if a PHP value matches a type.
///
/// It also tries to convert php_value to the type if it's not already the same type.
pub fn php_value_matches_type(
    r#type: &Type,
    php_value: &mut PhpValue,
    line: usize,
) -> Option<PhpError> {
    match r#type {
        Type::Named(_, _) => todo!(),
        Type::Nullable(_, r#type) => {
            if let PhpValue::Null = php_value {
                return None;
            }

            php_value_matches_type(r#type, php_value, line)
        }
        Type::Union(types) => {
            let matches_any = types
                .iter()
                .any(|ty| php_value_matches_type(ty, php_value, line).is_none());

            if !matches_any {
                return Some(errors::expected_type_but_got(
                    &types
                        .iter()
                        .map(|ty| ty.to_string())
                        .collect::<Vec<_>>()
                        .join("|"),
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
        Type::Intersection(types) => {
            for ty in types {
                if php_value_matches_type(ty, php_value, line).is_some() {
                    return Some(errors::expected_type_but_got(
                        &types
                            .iter()
                            .map(|ty| ty.to_string())
                            .collect::<Vec<_>>()
                            .join("&"),
                        php_value.get_type_as_string(),
                        line,
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
                    line,
                ));
            }

            None
        }
        Type::True(_) => {
            let PhpValue::Bool(b) = *php_value else {
                return Some(errors::expected_type_but_got(
                    "true",
                    php_value.get_type_as_string(),
                    line,
                ));
            };

            if !b {
                return Some(errors::expected_type_but_got(
                    "true",
                    php_value.get_type_as_string(),
                    line,
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
                    line,
                ));
            };

            if *b {
                return Some(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    line,
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
                    line,
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
                    line,
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
                    line,
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
                    line,
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
                    line,
                ));
            }

            None
        }
        Type::Object(_) => {
            if !matches!(php_value, PhpValue::Object(_)) {
                return Some(errors::expected_type_but_got(
                    "object",
                    php_value.get_type_as_string(),
                    line,
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
                    line,
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

pub fn parse_function_parameter_list(
    params: FunctionParameterList,
	evaluator: &mut Evaluator
) -> Result<Vec<CallableArgument>, PhpError> {
	let mut callable_args = vec![];

    for arg in params {
        let mut default_value: Option<PhpValue> = None;

        if arg.default.is_some() && arg.data_type.is_some() {
            let mut default_expression = evaluator.eval_expression(&arg.default.unwrap())?;
            let default_data_type = arg.data_type.clone().unwrap();

            // TODO: The data type conversion should not be done in this case
            // and only the actual data type should be accepted.
            let is_not_valid =
                php_value_matches_type(&default_data_type, &mut default_expression, 0);

            if is_not_valid.is_some() {
                return Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: format!(
                        "Cannot use {} as default value for parameter {} of type {}",
                        default_expression.get_type_as_string(),
                        get_string_from_bytes(&arg.name.name.bytes),
                        default_data_type
                    ),
                    line: default_data_type.first_span().line,
                });
            }

            default_value = Some(default_expression);
        }

        callable_args.push(CallableArgument {
            name: arg.name,
            data_type: arg.data_type,
            pass_by_reference: arg.ampersand.is_some(),
            default_value,
            ellipsis: arg.ellipsis.is_some(),
        });
    }

	Ok(callable_args)
}
