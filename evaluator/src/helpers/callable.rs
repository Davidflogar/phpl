use php_parser_rs::parser::ast::functions::FunctionParameterList;

use crate::{
    errors::{self, cannot_use_default_value_for_parameter},
    evaluator::Evaluator,
    php_value::{
        argument_type::PhpArgumentType,
        error::PhpError,
        primitive_data_types::{CallableArgument, PhpValue},
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
) -> Result<(), PhpError> {
    match r#type {
        PhpArgumentType::Nullable(r#type) => {
            if let PhpValue::Null = php_value {
                return Ok(());
            }

            php_value_matches_type(r#type, php_value, line)
        }
        PhpArgumentType::Union(types) => {
            let matches_any = types
                .iter()
                .any(|ty| php_value_matches_type(ty, php_value, line).is_ok());

            if !matches_any {
                return Err(errors::expected_type_but_got(
                    &types
                        .iter()
                        .map(|ty| ty.to_string())
                        .collect::<Vec<_>>()
                        .join("|"),
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Intersection(types) => {
            for ty in types {
                if php_value_matches_type(ty, php_value, line).is_err() {
                    return Err(errors::expected_type_but_got(
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

            Ok(())
        }
        PhpArgumentType::Null => {
            if !matches!(php_value, PhpValue::Null) {
                return Err(errors::expected_type_but_got(
                    "null",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::True => {
            let PhpValue::Bool(b) = *php_value else {
                return Err(errors::expected_type_but_got(
                    "true",
                    php_value.get_type_as_string(),
                    line,
                ));
            };

            if !b {
                return Err(errors::expected_type_but_got(
                    "true",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            *php_value = PhpValue::Bool(true);

            Ok(())
        }
        PhpArgumentType::False => {
            let PhpValue::Bool(b) = php_value else {
                return Err(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    line,
                ));
            };

            if *b {
                return Err(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            *php_value = PhpValue::Bool(false);

            Ok(())
        }
        PhpArgumentType::Float => {
            let to_float = php_value.to_float();

            if to_float.is_none() {
                return Err(errors::expected_type_but_got(
                    "float",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            *php_value = PhpValue::Float(to_float.unwrap());

            Ok(())
        }
        PhpArgumentType::Bool => {
            if !matches!(php_value, PhpValue::Bool(_)) {
                return Err(errors::expected_type_but_got(
                    "boolean",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Int => {
            let is_int = php_value.to_int();

            if is_int.is_none() {
                return Err(errors::expected_type_but_got(
                    "int",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            *php_value = PhpValue::Int(is_int.unwrap());

            Ok(())
        }
        PhpArgumentType::String => {
            let is_string = php_value.to_string();

            if is_string.is_none() {
                return Err(errors::expected_type_but_got(
                    "string",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            *php_value = PhpValue::String(is_string.unwrap().into());

            Ok(())
        }
        PhpArgumentType::Array => {
            if !matches!(php_value, PhpValue::Array(_)) {
                return Err(errors::expected_type_but_got(
                    "array",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Object => {
            if !matches!(php_value, PhpValue::Object(_)) {
                return Err(errors::expected_type_but_got(
                    "object",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Mixed => Ok(()),
        PhpArgumentType::Callable => {
            if !matches!(php_value, PhpValue::Callable(_)) {
                return Err(errors::expected_type_but_got(
                    "callable",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Iterable => todo!(),
        PhpArgumentType::StaticReference => unreachable!(),
        PhpArgumentType::SelfReference => todo!(),
        PhpArgumentType::ParentReference => todo!(),
        PhpArgumentType::Named(object) => {
            let PhpValue::Object(given_object) = php_value else {
				return Err(errors::expected_type_but_got(
					&object.get_name(),
					php_value.get_type_as_string(),
					line,
				));
			};

            let instance_of = given_object.instance_of(object);

            if !instance_of {
                return Err(errors::expected_type_but_got(
                    &object.get_name(),
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
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
        if declared_args.contains(&arg.name.name) {
            return Err(errors::redefinition_of_parameter(
                &arg.name.name,
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
                &PhpArgumentType::from_type(default_data_type, &evaluator.env.borrow_mut())?,
                &mut default_expression,
                0,
            );

            if is_not_valid.is_err() {
                return Err(cannot_use_default_value_for_parameter(
                    default_expression.get_type_as_string(),
                    get_string_from_bytes(&arg.name.name),
                    default_data_type.to_string(),
                    default_data_type.first_span().line,
                ));
            }

            default_value = Some(default_expression);
        }

        let mut data_type: Option<PhpArgumentType> = None;

        if let Some(arg_data_type) = arg.data_type {
            data_type = Some(PhpArgumentType::from_type(
                &arg_data_type,
                &evaluator.env.borrow_mut(),
            )?);
        };

        callable_args.push(CallableArgument {
            name: arg.name.clone(),
            data_type,
            default_value,
            is_variadic: arg.ellipsis.is_some(),
        });

        declared_args.push(arg.name.name);
    }

    Ok(callable_args)
}
