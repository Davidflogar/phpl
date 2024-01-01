use php_parser_rs::parser::ast::{identifiers::Identifier, Expression, NewExpression};

use crate::{
    evaluator::Evaluator,
    helpers::get_string_from_bytes,
    php_value::{
        error::{ErrorLevel, PhpError},
        objects::PhpObject,
        primitive_data_types::PhpValue,
    },
};

pub fn expression(
    evaluator: &mut Evaluator,
    expression: &NewExpression,
) -> Result<PhpValue, PhpError> {
    let target_name: Vec<u8>;

    if let Expression::Identifier(ref ident) = *expression.target {
        match ident {
            Identifier::SimpleIdentifier(i) => target_name = i.value.bytes.clone(),
            Identifier::DynamicIdentifier(_) => todo!(),
        }
    } else {
        let value = evaluator.eval_expression(&expression.target)?;

        let PhpValue::String(name) = value else {
            return Err(PhpError{
                level: ErrorLevel::Fatal,
				message: "Name must be a valid object or a string".to_string(),
				line: expression.new.line,
			});
        };

        target_name = name.bytes;
    }

    let Some(object) = evaluator.scope().get_object(&target_name) else {
		return Err(PhpError{
			level: ErrorLevel::Fatal,
			message: format!("Class {} not found", String::from_utf8_lossy(&target_name)),
			line: expression.new.line,
		});
	};

    if let PhpObject::AbstractClass(_) = object {
        return Err(PhpError {
            level: ErrorLevel::Fatal,
            message: format!(
                "Cannot instantiate abstract class {}",
                get_string_from_bytes(&target_name)
            ),
            line: expression.new.line,
        });
    }

    Ok(PhpValue::Null)
}
