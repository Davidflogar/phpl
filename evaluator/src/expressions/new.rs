use php_parser_rs::parser::ast::{identifiers::Identifier, Expression, NewExpression};

use crate::{
    evaluator::Evaluator,
    helpers::get_string_from_bytes,
    php_data_types::{
        error::{ErrorLevel, PhpError},
        objects::PhpObject,
        primitive_data_types::PhpValue,
    },
};

pub fn expression(
    evaluator: &mut Evaluator,
    expression: NewExpression,
) -> Result<PhpValue, PhpError> {
    let target_name: Vec<u8>;

    if let Expression::Identifier(ref ident) = *expression.target {
        match ident {
            Identifier::SimpleIdentifier(i) => target_name = i.value.bytes.clone(),
            Identifier::DynamicIdentifier(_) => todo!(),
        }
    } else {
        let value = evaluator.eval_expression(*expression.target)?;

        if !value.is_string() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: "Name must be a valid object or a string".to_string(),
                line: expression.new.line,
            });
        };

        let value_as_string = value.into_string();

        target_name = value_as_string.into_vec();
    }

    let Some(object) = evaluator.scope().get_object_cloned(&target_name) else {
        return Err(PhpError {
            level: ErrorLevel::Fatal,
            message: format!("Class {} not found", get_string_from_bytes(&target_name)),
            line: expression.new.line,
        });
    };

    let mut class = match object {
        PhpObject::Class(class) => class,
        PhpObject::AbstractClass(_) => {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot instantiate abstract class {}",
                    get_string_from_bytes(&target_name)
                ),
                line: expression.new.line,
            })
        }
        PhpObject::Trait(_) => {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot instantiate trait {}",
                    get_string_from_bytes(&target_name)
                ),
                line: expression.new.line,
            })
        }
    };

    class.call_constructor(evaluator, expression.arguments, expression.new)?;

    Ok(PhpValue::new_object(PhpObject::Class(class)))
}
