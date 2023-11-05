use std::io::Result;

use php_parser_rs::parser;

use crate::evaluator::Evaluator;

/// Evaluate the program.
pub fn eval_program(input: &str, content: &str) -> Result<()> {
    match parser::parse(content) {
        Ok(ast) => {
            let mut evaluator = Evaluator::new();

            for node in ast {
                let result = evaluator.eval_statement(node);

                if evaluator.die || result.is_err() {
                    if result.is_err() {
                        evaluator.output = format!("{}", result.unwrap_err().get_message(input));
                    }

                    break;
                }
            }

            for warning in evaluator.warnings {
                println!("{}", warning.get_message(input))
            }

            print!("{}", evaluator.output);
        }
        Err(err) => {
            println!("{}", err.report(&content, Some(input), true, false)?);
        }
    }

    return Ok(());
}
