use php_parser_rs::{lexer::token::Span, parser::{ast::variables::Variable, self}};

use crate::evaluator::Evaluator;

pub fn get_variable_span(var: Variable) -> Span {
    match var {
        Variable::SimpleVariable(v) => v.span,
        Variable::VariableVariable(vv) => vv.span,
        Variable::BracedVariableVariable(bvv) => bvv.start,
    }
}

/// Includes a file, this function is used with "require" and "include".
pub fn include_php_file(input: &str, content: &str) {
    match parser::parse(content) {
        Ok(ast) => {
            let mut evaluator = Evaluator::new();

            for node in ast {
                evaluator.eval_statement(node);

                if evaluator.die {
                    break;
                }
            }

            for warning in evaluator.warnings {
                println!("{}: {}", input, warning.message);
            }

            print!("{}", evaluator.output);
        }
        Err(err) => {
            println!("{}", err.report(&content, Some(input), true, false).unwrap());
        }
    }
}
