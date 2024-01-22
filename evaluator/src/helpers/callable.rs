use php_parser_rs::parser::ast::functions::FunctionParameterList;

use crate::{
    errors::{self, cannot_use_default_value_for_parameter},
    evaluator::Evaluator,
    php_value::{
        argument_type::PhpArgumentType,
        error::PhpError,
        primitive_data_types::{PhpFunctionArgument, PhpValue},
    },
};

/// Checks if a PHP value matches a type.
pub fn php_value_matches_type(
    r#type: &PhpArgumentType,
    php_value: &PhpValue,
    line: usize,
) -> Result<(), PhpError> {
    match r#type {
        PhpArgumentType::Nullable(r#type) => {
            if php_value.is_null() {
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
            if !php_value.is_null() {
                return Err(errors::expected_type_but_got(
                    "null",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::True => {
            let Some(b) = php_value.as_bool() else {
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

            Ok(())
        }
        PhpArgumentType::False => {
            let Some(b) = php_value.as_bool() else {
                return Err(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    line,
                ));
            };

            if b {
                return Err(errors::expected_type_but_got(
                    "false",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Float => {
            if !php_value.is_float() {
                return Err(errors::expected_type_but_got(
                    "float",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Bool => {
            if !php_value.is_bool() {
                return Err(errors::expected_type_but_got(
                    "boolean",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Int => {
            if !php_value.is_int() {
                return Err(errors::expected_type_but_got(
                    "int",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::String => {
            if !php_value.is_string() {
                return Err(errors::expected_type_but_got(
                    "string",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Array => {
            if !php_value.is_array() {
                return Err(errors::expected_type_but_got(
                    "array",
                    php_value.get_type_as_string(),
                    line,
                ));
            }

            Ok(())
        }
        PhpArgumentType::Object => {
            if !php_value.is_object() {
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
            if !php_value.is_callable() {
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

pub fn eval_function_parameter_list(
    params: FunctionParameterList,
    evaluator: &mut Evaluator,
) -> Result<Vec<PhpFunctionArgument>, PhpError> {
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
            let result_of_default_value = evaluator.eval_expression(arg.default.unwrap())?;
            let argument_data_type = arg.data_type.as_ref().unwrap();

            // TODO: The data type conversion should not be done in this case
            // and only the actual data type should be accepted.
            let is_not_valid = php_value_matches_type(
                &PhpArgumentType::from_type(argument_data_type, &evaluator.scope())?,
                &result_of_default_value,
                0,
            );

            if is_not_valid.is_err() {
                return Err(cannot_use_default_value_for_parameter(
                    result_of_default_value.get_type_as_string(),
                    arg.name.name.to_string(),
                    argument_data_type.to_string(),
                    argument_data_type.first_span().line,
                ));
            }

            default_value = Some(result_of_default_value);
        } else if arg.default.is_some() && arg.data_type.is_none() {
            default_value = Some(evaluator.eval_expression(arg.default.unwrap())?);
        }

        let mut data_type: Option<PhpArgumentType> = None;

        if let Some(arg_data_type) = arg.data_type {
            data_type = Some(PhpArgumentType::from_type(
                &arg_data_type,
                &evaluator.scope(),
            )?);
        };

        callable_args.push(PhpFunctionArgument {
            name: arg.name.clone(),
            data_type,
            default_value,
            is_variadic: arg.ellipsis.is_some(),
            pass_by_reference: arg.ampersand.is_some(),
        });

        declared_args.push(arg.name.name);
    }

    Ok(callable_args)
}
