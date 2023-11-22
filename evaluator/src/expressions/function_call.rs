use std::collections::HashMap;

use php_parser_rs::parser::ast::{arguments::Argument, FunctionCallExpression};

use crate::{
    evaluator::Evaluator,
    helpers::helpers::get_string_from_bytes,
    php_value::php_value::{CallableArgument, ErrorLevel, PhpError, PhpValue},
};

pub fn expression(
    evaluator: &mut Evaluator,
    call: &FunctionCallExpression,
) -> Result<PhpValue, PhpError> {
    let called_in_line = call.arguments.left_parenthesis.line;

    // get the target
    let target = evaluator.eval_expression(&call.target)?;

    let target_to_string = target.printable();

    if target_to_string.is_none() {
        evaluator.warnings.push(PhpError {
            level: ErrorLevel::Warning,
            message: format!(
                "{} to string conversion failed",
                target.get_type_as_string()
            ),
            line: called_in_line,
        });
    }

    let target_name = target_to_string.unwrap();

    let target_name_as_vec = target_name.as_bytes().to_vec();

    let function_option = evaluator.env.get_ident(&target_name_as_vec);

    if function_option.is_none() {
        let error = format!("Function {} not found", target_name);

        return Err(PhpError {
            level: ErrorLevel::Fatal,
            message: error,
            line: called_in_line,
        });
    }

    let PhpValue::Callable(function) = function_option.unwrap() else {
		let error = format!("Type {} is not callable", target_name);

		return Err(PhpError { level: ErrorLevel::Fatal, message: error, line:  called_in_line});
	};

    let mut final_function_parameters: HashMap<Vec<u8>, PhpValue> = HashMap::new();

    if function.parameters.len() != 0 {
        let mut required_arguments: Vec<&CallableArgument> = vec![];

        // get the arguments that are required by the function,
        // even if they have a default value
        for arg in &function.parameters {
            required_arguments.push(arg);
        }

        // get the arguments with which the function was called
        let mut positional_arguments = Vec::new();
        let mut named_arguments = HashMap::new();

        for argument in &call.arguments.arguments {
            match argument {
                Argument::Positional(positional_arg) => {
                    let argument_value = evaluator.eval_expression(&positional_arg.value)?;

                    if positional_arg.ellipsis.is_some() {
                        if !argument_value.is_iterable() {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: "Only arrays and Traversables can be unpacked".to_string(),
                                line: called_in_line,
                            });
                        }

                        todo!()
                    }

                    positional_arguments.push(argument_value);
                }
                Argument::Named(named_arg) => {
                    let name = &named_arg.name.value;
                    let value = evaluator.eval_expression(&named_arg.value)?;

                    if named_arg.ellipsis.is_some() {
                        if !value.is_iterable() {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: "Only arrays and Traversables can be unpacked".to_string(),
                                line: called_in_line,
                            });
                        }

                        todo!()
                    }

                    named_arguments.insert(name.to_vec(), value);
                }
            }
        }

        // iterate over the positional arguments and check if they are valid
        for (i, mut positional_arg) in positional_arguments.into_iter().enumerate() {
            if i > function.parameters.len() - 1 {
                break;
            }

            let self_arg = &function.parameters[i];

            // validate the argument
            let is_not_valid = self_arg.is_valid(&mut positional_arg, called_in_line);

            if is_not_valid.is_some() {
                let mut error = is_not_valid.unwrap();

                error.message = format!(
                    "{}(): Argument #{} ({}): {}",
                    target_name,
                    i + 1,
                    get_string_from_bytes(&self_arg.name.name.bytes),
                    error.message
                );

                return Err(error);
            }

            final_function_parameters.insert(self_arg.name.name.to_vec(), positional_arg.clone());

            required_arguments.retain(|c| c.name.name != self_arg.name.name);
        }

        // iterate over the named arguments and check if they are valid
        for (mut key, mut value) in named_arguments {
            // add the $ at the beginning since the argument name is saved with $
            key.insert(0, b'$');

            if final_function_parameters.contains_key(&key) {
                return Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: format!(
                        "Named argument {} overwrites previous argument, called in",
                        get_string_from_bytes(&key)
                    ),
                    line: called_in_line,
                });
            }

            let mut self_arg: Option<&CallableArgument> = None;

            if !function.parameters.iter().any(|c| {
                if c.name.name.bytes == key {
                    self_arg = Some(c);

                    return true;
                }

                c.name.name.bytes == key
            }) {
                return Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: format!("Unknown named argument {}", get_string_from_bytes(&key)),
                    line: called_in_line,
                });
            }

            // validate the argument
            let self_arg = self_arg.unwrap();

            let is_not_valid = self_arg.is_valid(&mut value, called_in_line);

            if is_not_valid.is_some() {
                let mut error = is_not_valid.unwrap();

                error.message = format!(
                    "{}(): Argument #{} ({}): {}",
                    target_name,
                    final_function_parameters.len(),
                    get_string_from_bytes(&self_arg.name.name.bytes),
                    error.message
                );

                return Err(error);
            }

            required_arguments.retain(|c| c.name.name.bytes != key);

            final_function_parameters.insert(key, value);
        }

        for noa in required_arguments.iter() {
            if noa.default_value.is_none() {
                return Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: format!(
                        "Too few arguments to function {}(), {} passed and exactly {} was expected",
                        target_name,
                        call.arguments.arguments.len(),
                        required_arguments.len() + 1
                    ),
                    line: called_in_line,
                });
            }

            final_function_parameters
                .insert(noa.name.name.to_vec(), noa.default_value.clone().unwrap());
        }
    }

    // start the trace of the environment
    evaluator.env.start_trace();

    for new_var in final_function_parameters {
        evaluator.env.insert_var(&new_var.0, &new_var.1);
    }

    // execute the function
    let mut result: Result<PhpValue, PhpError> = Ok(PhpValue::Null);

    for statement in function.body {
        result = evaluator.eval_statement(statement);
    }

    // restore the environment
    evaluator.env.restore();

    if result.is_err() {
        let mut err = result.unwrap_err();
        err.line = called_in_line;

        return Err(err);
    }

    return result;
}
