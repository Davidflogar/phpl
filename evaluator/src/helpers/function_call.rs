use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use php_parser_rs::parser::ast::{arguments::Argument, Statement};

use crate::{
    errors::{redefinition_of_parameter, too_few_arguments_to_function},
    evaluator::Evaluator,
    helpers::get_string_from_bytes,
    php_data_types::{
        error::{ErrorLevel, PhpError},
        primitive_data_types::{PhpFunctionArgument, PhpValue},
    },
    scope::Scope,
};

pub fn generic_function_call(
    evaluator: &mut Evaluator,
    target_name: String,
    function_arguments: &Vec<PhpFunctionArgument>,
    function_call_arguments: Vec<Argument>,
    called_in_line: usize,
    function_body: Vec<Statement>,
) -> Result<PhpValue, PhpError> {
    // prepare the needed vars
    let mut parameters_to_pass_to_the_function = HashMap::new();

    let function_call_arguments_len = function_call_arguments.len();

    if !function_arguments.is_empty() {
        let function_parameters_len = function_arguments.len();

        // get the arguments that are required by the function,
        // even if they have a default value
        let mut required_arguments = VecDeque::new();

        for arg in function_arguments {
            required_arguments.push_back(arg);
        }

        for (position, argument) in function_call_arguments.into_iter().enumerate() {
            match argument {
                Argument::Positional(positional_argument) => {
                    if position > function_parameters_len - 1 {
                        break;
                    }

                    let function_argument = required_arguments.pop_front().unwrap();

                    if parameters_to_pass_to_the_function
                        .contains_key(&function_argument.name.bytes)
                    {
                        return Err(redefinition_of_parameter(
                            &function_argument.name,
                            called_in_line,
                        ));
                    }

                    // validate the argument
                    let validation_result = function_argument
                        .must_be_valid(evaluator, Argument::Positional(positional_argument));

                    if let Err((error, error_string)) = validation_result {
                        if error.is_none() {
                            let error = PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
                                    "{}(): Argument #{} ({}): {}",
                                    target_name,
                                    position + 1,
                                    get_string_from_bytes(&function_argument.name),
                                    error_string
                                ),
                                line: called_in_line,
                            };

                            return Err(error);
                        }

                        return Err(error.unwrap());
                    }

                    parameters_to_pass_to_the_function
                        .insert(function_argument.name.to_vec(), validation_result.unwrap());
                }
                Argument::Named(named_argument) => {
                    let mut argument_name = named_argument.name.value.clone();

                    // add the $ at the beginning
                    // since the arguments inside required_arguments are saved with the $ at the beginning
                    argument_name.bytes.insert(0, b'$');

                    if parameters_to_pass_to_the_function.contains_key(&argument_name.bytes) {
                        return Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: format!(
                                "Named argument {} overwrites previous argument",
                                get_string_from_bytes(&argument_name)
                            ),
                            line: named_argument.name.span.line,
                        });
                    }

                    let argument_position_some = required_arguments
                        .iter()
                        .position(|c| c.name == argument_name);

                    let Some(argument_position) = argument_position_some else {
						return Err(PhpError {
							level: ErrorLevel::Fatal,
							message: format!(
								"Unknown named argument {}",
								get_string_from_bytes(&argument_name)
							),
							line: named_argument.name.span.line,
						})
					};

                    let function_arg = required_arguments.remove(argument_position).unwrap();

                    // from here it is basically the same as working with a positional argument.
                    let validation_result =
                        function_arg.must_be_valid(evaluator, Argument::Named(named_argument));

                    if let Err((error, error_string)) = validation_result {
                        if error.is_none() {
                            let error = PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
                                    "{}(): Argument #{} ({}): {}",
                                    target_name,
                                    position + 1,
                                    get_string_from_bytes(&function_arg.name),
                                    error_string
                                ),
                                line: called_in_line,
                            };

                            return Err(error);
                        }

                        return Err(error.unwrap());
                    }

                    parameters_to_pass_to_the_function
                        .insert(function_arg.name.to_vec(), validation_result.unwrap());
                }
            }
        }

        let required_arguments_len = required_arguments.len();

        for required_arg in required_arguments {
            let Some(ref default_value) = required_arg.default_value else {
                return Err(too_few_arguments_to_function(
					target_name,
					function_call_arguments_len,
					required_arguments_len,
					called_in_line,
				));
            };

            parameters_to_pass_to_the_function
                .insert(required_arg.name.to_vec(), default_value.clone());
        }
    }

    let old_scope = Rc::clone(&evaluator.scope);

    let new_scope = Scope::new();

    evaluator.change_scope(Rc::new(RefCell::new(new_scope)));

    for new_var in parameters_to_pass_to_the_function {
        evaluator.scope().set_var_value(&new_var.0, new_var.1);
    }

    let mut error = None;

    // execute the function
    for statement in function_body {
        if let Err(err) = evaluator.eval_statement(statement) {
            error = Some(err);
            break;
        }
    }

    // change to the old environment
    evaluator.change_scope(old_scope);

    if let Some(err) = error {
        return Err(err);
    }

    // TODO: return a value from the function
    Ok(PhpValue::Null)
}
