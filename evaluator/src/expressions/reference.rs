use php_parser_rs::parser::ast::{Expression, ReferenceExpression};

use crate::{
    evaluator::Evaluator,
    php_data_types::{
        error::{ErrorLevel, PhpError},
        primitive_data_types::PhpValue,
    },
};

pub fn expression(
    evaluator: &mut Evaluator,
    reference: ReferenceExpression,
) -> Result<PhpValue, (PhpError, bool)> {
    match *reference.right {
        Expression::Variable(variable) => {
            let result = evaluator.get_variable_name(variable);

            let Ok(var_name) = result else {
                return Err((result.unwrap_err(), false));
            };

            Ok(evaluator.scope().new_ref(var_name))
        }
        _ => Err((
            PhpError {
                level: ErrorLevel::Fatal,
                message: "Invalid reference expression".to_string(),
                line: reference.ampersand.line,
            },
            true,
        )),
    }
}
