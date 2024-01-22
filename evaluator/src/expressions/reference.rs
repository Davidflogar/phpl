use php_parser_rs::parser::ast::{Expression, ReferenceExpression};

use crate::{
    evaluator::Evaluator,
    php_value::{
        error::{ErrorLevel, PhpError},
        primitive_data_types::PhpValue,
    },
};

pub fn expression(
    evaluator: &mut Evaluator,
    reference: ReferenceExpression,
) -> Result<PhpValue, PhpError> {
    match *reference.right {
        Expression::Variable(variable) => {
            let var_name = evaluator.get_variable_name(variable)?;

            Ok(PhpValue::Reference(evaluator.scope().new_ref(&var_name)))
        }
        _ => Err(PhpError {
            level: ErrorLevel::Fatal,
            message: "Invalid reference expression".to_string(),
            line: reference.ampersand.line,
        }),
    }
}
