use std::{cell::RefCell, io::Result as IoResult, rc::Rc};

use php_parser_rs::parser;

use crate::{evaluator::Evaluator, scope::Scope};

/// Evaluate the program.
pub fn eval_program(input: &str, content: &str) -> IoResult<()> {
    match parser::parse(content) {
        Ok(ast) => {
            let env = Scope::new();

            let mut evaluator = Evaluator::new(Rc::new(RefCell::new(env)));

            for node in ast {
                let result = evaluator.eval_statement(node);

                if evaluator.die || result.is_err() {
                    if let Err(error) = result {
                        evaluator.output = error.get_message(input)
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
            println!("{}", err.report(content, Some(input), true, false)?);
        }
    }

    Ok(())
}
