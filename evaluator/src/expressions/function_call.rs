use php_parser_rs::parser::ast::{
    identifiers::Identifier, Expression, FunctionCallExpression, Statement,
};

use crate::{
    errors::type_is_not_callable,
    evaluator::Evaluator,
    helpers::{function_call, get_string_from_bytes},
    php_data_types::{
        error::{ErrorLevel, PhpError},
        primitive_data_types::{PhpFunctionArgument, PhpIdentifier, PhpValue},
    },
};

pub fn expression(
    evaluator: &mut Evaluator,
    call: FunctionCallExpression,
) -> Result<PhpValue, PhpError> {
    let called_in_line = call.arguments.left_parenthesis.line;

    // get the function body and params
    let target_name: String;

    let function_arguments: Vec<PhpFunctionArgument>;
    let function_body: Vec<Statement>;

    if let Expression::Identifier(identifier) = *call.target {
        let scope = evaluator.scope.borrow();

        match identifier {
            Identifier::SimpleIdentifier(simple_identifier) => {
                let Some(identifier_value) = scope.get_ident(&simple_identifier.value) else {
                    return Err(PhpError {
                        level: ErrorLevel::Fatal,
                        message: format!(
                            "Call to undefined function {}()",
                            simple_identifier.value
                        ),
                        line: called_in_line,
                    });
                };

                let PhpIdentifier::Function(ref borrowed_function) = identifier_value else {
                    return Err(PhpError {
                        level: ErrorLevel::Fatal,
                        message: format!(
                            "{}(): Call to undefined function",
                            simple_identifier.value
                        ),
                        line: called_in_line,
                    });
                };

                target_name = get_string_from_bytes(&simple_identifier.value);
                function_arguments = borrowed_function.parameters.clone();
                function_body = borrowed_function.body.clone();
            }
            Identifier::DynamicIdentifier(_) => todo!(),
        }
    } else {
        let expression_result = evaluator.eval_expression(*call.target)?;

        if !expression_result.is_string() {
            return Err(type_is_not_callable(
                expression_result.get_type_as_string(),
                called_in_line,
            ));
        };

        let function_name_as_bytes = expression_result.as_string();

        let function_name = get_string_from_bytes(function_name_as_bytes.as_ref());

        let scope = evaluator.scope.borrow();

        let Some(identifier_value) = scope.get_ident(function_name_as_bytes.as_ref()) else {
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
        function_arguments = borrowed_function.parameters.clone();
        function_body = borrowed_function.body.clone();
    }

    function_call::generic_function_call(
        evaluator,
        target_name,
        &function_arguments,
        call.arguments.arguments,
        called_in_line,
        function_body,
    )
}
