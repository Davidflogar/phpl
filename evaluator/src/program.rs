use std::io::Result;

use php_parser_rs::parser;

use crate::evaluator::Evaluator;

/// Evaluate the program.
pub fn eval_program(content: String) -> Result<()> {
    match parser::parse(&content) {
        Ok(ast) => {
            let mut evaluator = Evaluator::new();

            for node in ast {
                evaluator.eval_statement(node);

                if evaluator.die {
                    if evaluator.error.message != "" {
                        evaluator.output = evaluator.error.message;
                    }

                    break;
                }
            }

			for warning in evaluator.warnings {
				println!("{}", warning.message);
			}

            print!("{}", evaluator.output);
        }
        Err(err) => {
            println!("{}", err.report(&content, None, true, false)?);
        }
    }

    return Ok(());
}
