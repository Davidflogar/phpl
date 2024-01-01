use std::{collections::HashMap, hash::Hash};

use php_parser_rs::{
    lexer::token::Span,
    parser::{
        self,
        ast::{
            modifiers::{MethodModifier, VisibilityModifier},
            variables::Variable,
        },
    },
};

use crate::{
    evaluator::Evaluator,
    php_value::{
        error::{ErrorLevel, PhpError},
        primitive_data_types::PhpValue,
    },
};

pub mod callable;
pub mod object;

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
            let mut last_result = PhpValue::Null;

            for node in ast {
                let result = evaluator.eval_statement(node);

                if evaluator.die || result.is_err() {
                    if let Err(error) = result {
                        evaluator.output = error.get_message(input);
                    }

                    break;
                }

                last_result = result.unwrap();
            }

            Ok(last_result)
        }
        Err(err) => {
            let err = err.report(content, Some(input), true, false);

            if err.is_err() {
                panic!("{}", err.unwrap_err());
            }

            Err(PhpError {
                level: ErrorLevel::Raw,
                message: format!("PHP Parse Error in {}: {}", input, err.unwrap()),
                line: 0,
            })
        }
    }
}

pub fn get_string_from_bytes(var: &[u8]) -> String {
    String::from_utf8_lossy(var).to_string()
}

pub fn extend_hashmap_without_overwrite<K, V>(map: &mut HashMap<K, V>, other: HashMap<K, V>)
where
    K: Eq + Hash,
{
    for (key, value) in other {
        map.entry(key).or_insert(value);
    }
}

pub fn visibility_modifier_to_method_modifier(visibility: &VisibilityModifier) -> MethodModifier {
    match visibility {
        VisibilityModifier::Public(span) => MethodModifier::Public(*span),
        VisibilityModifier::Protected(span) => MethodModifier::Protected(*span),
        VisibilityModifier::Private(span) => MethodModifier::Private(*span),
    }
}
