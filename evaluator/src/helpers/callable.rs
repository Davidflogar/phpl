use php_parser_rs::parser::ast::functions::FunctionParameterList;

use crate::{
    errors::{self, cannot_use_default_value_for_parameter},
    evaluator::Evaluator,
    php_value::{
        php_argument_type::PhpArgumentType,
        primitive_data_types::{CallableArgument, PhpError, PhpValue},
    },
};

use super::get_string_from_bytes;

/// Checks if a PHP value matches a type.
///
/// It also tries to convert php_value to the type if it's not already the same type.
pub fn php_value_matches_type(
    r#type: &PhpArgumentType,
    php_value: &mut PhpValue,
    line: usize,
) -> Option<PhpError> {
    match r#type {
        PhpArgumentType::Nullable(r#type) => {
            if let PhpValue::Null = php_value {
                return None;
            }

            php_value_matches_type(r#type, php_value, line)
        }
        PhpArgumentType::Union(types) => {
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
        PhpArgumentType::Intersection(types) => {
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
        PhpArgumentType::Null => {
            if !matches!(php_value, PhpValue::Null) {
                return Some(errors::expected_type_but_got(
                    "null",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
        PhpArgumentType::True => {
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
        PhpArgumentType::False => {
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
        PhpArgumentType::Float => {
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
        PhpArgumentType::Bool => {
            if !matches!(php_value, PhpValue::Bool(_)) {
                return Some(errors::expected_type_but_got(
                    "boolean",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
        PhpArgumentType::Int => {
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
        PhpArgumentType::String => {
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
        PhpArgumentType::Array => {
            if !matches!(php_value, PhpValue::Array(_)) {
                return Some(errors::expected_type_but_got(
                    "array",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
        PhpArgumentType::Object => {
            if !matches!(php_value, PhpValue::Object(_)) {
                return Some(errors::expected_type_but_got(
                    "object",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
        PhpArgumentType::Mixed => None,
        PhpArgumentType::Callable => {
            if !matches!(php_value, PhpValue::Callable(_)) {
                return Some(errors::expected_type_but_got(
                    "callable",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
        PhpArgumentType::Iterable => todo!(),
        PhpArgumentType::StaticReference => unreachable!(),
        PhpArgumentType::SelfReference => todo!(),
        PhpArgumentType::ParentReference => todo!(),
        PhpArgumentType::Named(object) => {
            let PhpValue::Object(given_object) = php_value else {
				return Some(errors::expected_type_but_got(
					&object.get_name(),
					php_value.get_type_as_string(),
					line,
				));
			};

            let instance_of = given_object.instance_of(object);

            if !instance_of {
                return Some(errors::expected_type_but_got(
                    &object.get_name(),
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            None
        }
    }
}

pub fn parse_function_parameter_list(
    params: FunctionParameterList,
    evaluator: &mut Evaluator,
) -> Result<Vec<CallableArgument>, PhpError> {
    let mut callable_args = vec![];
    let mut declared_args = vec![];

    for arg in params {
        if declared_args.contains(&arg.name.name.bytes) {
            return Err(errors::redefinition_of_parameter(
                &arg.name.name.bytes,
                arg.name.span.line,
            ));
        }

        let mut default_value = None;

        if arg.default.is_some() && arg.data_type.is_some() {
            let mut default_expression = evaluator.eval_expression(&arg.default.unwrap())?;
            let default_data_type = arg.data_type.as_ref().unwrap();

            // TODO: The data type conversion should not be done in this case
            // and only the actual data type should be accepted.
            let is_not_valid = php_value_matches_type(
                &PhpArgumentType::from_type(default_data_type, evaluator.env)?,
                &mut default_expression,
                0,
            );

            if is_not_valid.is_some() {
                return Err(cannot_use_default_value_for_parameter(
                    default_expression.get_type_as_string(),
                    get_string_from_bytes(&arg.name.name.bytes),
                    default_data_type.to_string(),
                    default_data_type.first_span().line,
                ));
            }

            default_value = Some(default_expression);
        }

        let mut data_type: Option<PhpArgumentType> = None;

        if let Some(arg_data_type) = arg.data_type {
            data_type = Some(PhpArgumentType::from_type(&arg_data_type, evaluator.env)?);
        };

        callable_args.push(CallableArgument {
            name: arg.name.clone(),
            data_type,
            default_value,
            is_variadic: arg.ellipsis.is_some(),
        });

        declared_args.push(arg.name.name.bytes);
    }

    Ok(callable_args)
}
