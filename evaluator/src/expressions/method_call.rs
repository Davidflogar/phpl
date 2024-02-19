use php_parser_rs::parser::ast::MethodCallExpression;

use crate::{
    evaluator::Evaluator,
    php_data_types::{
        error::PhpError,
        primitive_data_types::PhpValue,
    },
};

pub fn expression(
    evaluator: &mut Evaluator,
    method_call: MethodCallExpression,
) -> Result<PhpValue, PhpError> {
    let target_value = evaluator.eval_expression(*method_call.target)?;

	match target_value {
	}

    Ok(PhpValue::Null)
}
