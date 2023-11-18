use core::panic;

use php_parser_rs::{
    lexer::token::Span,
    parser::{self, ast::variables::Variable},
};

use crate::{
    evaluator::Evaluator,
    php_value::{PhpError, PhpValue},
};

pub fn get_span_from_var(var: &Variable) -> Span {
    match var {
        Variable::SimpleVariable(v) => v.span,
        Variable::VariableVariable(vv) => vv.span,
        Variable::BracedVariableVariable(bvv) => bvv.start,
    }
}

/// Parses a PHP file and returns the result, this function is used with "require" and "include".
pub fn parse_php_file(
    evaluator: &mut Evaluator,
    input: &str,
    content: &str,
) -> Result<PhpValue, PhpError> {
    match parser::parse(content) {
        Ok(ast) => {
            let mut child_evalutor = Evaluator::new(evaluator.env);

            let mut last_result = PhpValue::Null;

            for node in ast {
                let result = child_evalutor.eval_statement(node);

                if child_evalutor.die || result.is_err() {
                    if result.is_err() {
                        evaluator.output = format!("{}", result.unwrap_err().get_message(input));
                    }

                    break;
                }

                last_result = result.unwrap();
            }

            for warning in child_evalutor.warnings {
                // Note that here, although the error is a warning,
                // it is converted to an ErrorLevel::Raw so that
                // the error is not modified when calling get_message() twice on the same error.

                let new_warning = PhpError {
                    level: crate::php_value::ErrorLevel::Raw,
                    message: format!(
                        "PHP Warning: {} in {} on line {}",
                        warning.message, input, warning.line
                    ),
                    line: 0,
                };

                evaluator.warnings.push(new_warning);
            }

            evaluator.output += child_evalutor.output.as_str();

            Ok(last_result)
        }
        Err(err) => {
            let err = err.report(&content, Some(input), false, false);

            if err.is_err() {
                panic!("{}", err.unwrap_err());
            }

            Err(PhpError {
                level: crate::php_value::ErrorLevel::Raw,
                message: format!("PHP Parse Error in {}: {}", input, err.unwrap()),
                line: 0,
            })
        }
    }
}

pub fn get_string_from_bytes(var: &[u8]) -> String {
    String::from_utf8_lossy(var).to_string()
}
