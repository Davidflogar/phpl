use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use php_parser_rs::{
    lexer::token::Span,
    parser::ast::{
        arguments::Argument, identifiers::Identifier, Expression, FunctionCallExpression,
        ReferenceExpression, Statement,
    },
};

use crate::{
    errors::{only_arrays_and_traversables_can_be_unpacked, type_is_not_callable},
    evaluator::Evaluator,
    helpers::get_string_from_bytes,
    php_value::{
        error::{ErrorLevel, PhpError},
        primitive_data_types::{PhpFunctionArgument, PhpIdentifier, PhpValue},
    },
    scope::Scope,
};

use super::reference;

pub fn expression(
    evaluator: &mut Evaluator,
    call: FunctionCallExpression,
) -> Result<PhpValue, PhpError> {
    let called_in_line = call.arguments.left_parenthesis.line;

    // get the function body and params
    let target_name: String;

    let mut function_parameters: Vec<PhpFunctionArgument>;
    let function_body: Vec<Statement>;

    if let Expression::Identifier(identifier) = *call.target {
        let scope = evaluator.scope.borrow();

        match identifier {
            Identifier::SimpleIdentifier(simple_identifier) => {
                let Some(identifier_value) = scope.get_ident(&simple_identifier.value) else {
					return Err(PhpError {
						level: ErrorLevel::Fatal,
						message: format!("Call to undefined function {}()", simple_identifier.value),
						line: called_in_line,
					});
				};

                let PhpIdentifier::Function(ref borrowed_function) = identifier_value else {
					return Err(PhpError {
						level: ErrorLevel::Fatal,
						message: format!("{}(): Call to undefined function", simple_identifier.value),
						line: called_in_line,
					});
				};

                target_name = get_string_from_bytes(&simple_identifier.value);
                function_parameters = borrowed_function.parameters.clone();
                function_body = borrowed_function.body.clone();
            }
            Identifier::DynamicIdentifier(_) => todo!(),
        }
    } else {
        let expression_result = evaluator.eval_expression(*call.target)?;

        let PhpValue::String(function_name_as_bytes) = expression_result else {
			return Err(type_is_not_callable(expression_result.get_type_as_string(), called_in_line))
		};

        let function_name = get_string_from_bytes(&function_name_as_bytes);

        let scope = evaluator.scope.borrow();

        let Some(identifier_value) = scope.get_ident(&function_name_as_bytes) else {
			return Err(PhpError {
				level: ErrorLevel::Fatal,
				message: format!("Call to undefined function {}()", function_name),
				line: called_in_line,
			});
		};

        let PhpIdentifier::Function(ref borrowed_function) = identifier_value else {
			return Err(PhpError {
				level: ErrorLevel::Fatal,
				message: format!("{}(): Call to undefined function", function_name),
				line: called_in_line,
			});
		};

        target_name = function_name;
        function_parameters = borrowed_function.parameters.clone();
        function_body = borrowed_function.body.clone();
    }

    // prepare the needed vars

    let mut final_function_parameters: HashMap<Vec<u8>, PhpValue> = HashMap::new();

    let function_call_arguments = call.arguments.arguments.len();

    if !function_parameters.is_empty() {
        let function_parameters_len = function_parameters.len();

        // get the arguments that are required by the function,
        // even if they have a default value
        let mut required_arguments = VecDeque::new();

        required_arguments.extend(function_parameters.drain(..));

        for (position, argument) in call.arguments.arguments.into_iter().enumerate() {
            match argument {
                Argument::Positional(positional_argument) => {
                    if position > function_parameters_len - 1 {
                        break;
                    }

                    let function_arg = required_arguments.pop_front().unwrap();

                    let argument_value = if function_arg.pass_by_reference {
                        let unused_span = Span {
                            line: 0,
                            column: 0,
                            position: 0,
                        };

                        let reference_expression = ReferenceExpression {
                            ampersand: unused_span,
                            right: Box::new(positional_argument.value),
                        };

                        let expression_result =
                            reference::expression(evaluator, reference_expression);

                        let Ok(result) = expression_result else {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
									"{}(): Argument #{} ({}): could not be passed by reference",
									target_name,
									position + 1,
									get_string_from_bytes(&function_arg.name.name),
								),
                                line: called_in_line,
                            });
                        };

                        result
                    } else {
                        evaluator.eval_expression(positional_argument.value)?
                    };

                    if let Some(ellipsis) = positional_argument.ellipsis {
                        if !argument_value.is_iterable() {
                            return Err(only_arrays_and_traversables_can_be_unpacked(
                                ellipsis.line,
                            ));
                        }

                        todo!()
                    }

                    if function_arg.is_variadic {
                        todo!()
                    }

                    // validate the argument
                    let is_not_valid = function_arg.is_valid(&argument_value, called_in_line);

                    if let Err(mut error) = is_not_valid {
                        error.message = format!(
                            "{}(): Argument #{} ({}): {}",
                            target_name,
                            position + 1,
                            get_string_from_bytes(&function_arg.name.name),
                            error.message
                        );

                        return Err(error);
                    }

                    final_function_parameters
                        .insert(function_arg.name.name.to_vec(), argument_value);
                }
                Argument::Named(named_argument) => {
                    let mut argument_name = named_argument.name.value;

                    // add the $ at the beginning
                    // since the arguments inside required_arguments are saved with the $ at the beginning
                    argument_name.bytes.insert(0, b'$');

                    if final_function_parameters.contains_key(&argument_name.bytes) {
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
                        .position(|c| c.name.name == argument_name);

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

                    let argument_value = if function_arg.pass_by_reference {
                        let unused_span = Span {
                            line: 0,
                            column: 0,
                            position: 0,
                        };

                        let reference_expression = ReferenceExpression {
                            ampersand: unused_span,
                            right: Box::new(named_argument.value),
                        };

                        let expression_result =
                            reference::expression(evaluator, reference_expression);

                        let Ok(result) = expression_result else {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
									"{}(): Argument #{} ({}): could not be passed by reference",
									target_name,
									position + 1,
									get_string_from_bytes(&function_arg.name.name),
								),
                                line: called_in_line,
                            });
                        };

                        result
                    } else {
                        evaluator.eval_expression(named_argument.value)?
                    };

                    if let Some(ellipsis) = named_argument.ellipsis {
                        if !argument_value.is_iterable() {
                            return Err(only_arrays_and_traversables_can_be_unpacked(
                                ellipsis.line,
                            ));
                        }

                        todo!()
                    }

                    if function_arg.is_variadic {
                        todo!()
                    }

                    // validate the argument
                    let is_not_valid = function_arg.is_valid(&argument_value, called_in_line);

                    if let Err(mut error) = is_not_valid {
                        error.message = format!(
                            "{}(): Argument #{} ({}): {}",
                            target_name,
                            position + 1,
                            get_string_from_bytes(&function_arg.name.name),
                            error.message
                        );

                        return Err(error);
                    }

                    final_function_parameters
                        .insert(function_arg.name.name.to_vec(), argument_value);
                }
            }
        }

        let required_arguments_len = required_arguments.len();

        for required_arg in required_arguments {
            let Some(default_value) = required_arg.default_value else {
                return Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: format!(
                        "Too few arguments to function {}(), {} passed and exactly {} was expected",
                        target_name,
                        function_call_arguments,
                        required_arguments_len + 1
                    ),
                    line: called_in_line,
                });
            };

            final_function_parameters.insert(required_arg.name.name.to_vec(), default_value);
        }
    }

    let old_scope = Rc::clone(&evaluator.scope);

    let new_scope = Scope::new();

    evaluator.change_scope(Rc::new(RefCell::new(new_scope)));

    for new_var in final_function_parameters {
        evaluator.scope().set_var_value(&new_var.0, new_var.1);
    }

    // execute the function
    let mut result = Ok(PhpValue::Null);

    for statement in function_body {
        result = evaluator.eval_statement(statement);
    }

    // change to the old environment

    evaluator.change_scope(old_scope);

    if let Err(mut err) = result {
        err.line = called_in_line;

        return Err(err);
    }

    result
}
