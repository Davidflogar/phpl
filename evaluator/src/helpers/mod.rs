use std::{collections::HashMap, hash::Hash};

use php_parser_rs::{
    lexer::token::Span,
    parser::{
        self,
        ast::{
            identifiers::Identifier,
            modifiers::{MethodModifier, VisibilityModifier},
            variables::Variable,
        },
    },
};

use crate::{
    evaluator::Evaluator,
    php_data_types::{
        argument_type::PhpArgumentType,
        error::{ErrorLevel, PhpError},
        primitive_data_types::PhpValue,
    },
};

pub mod callable;
pub mod function_call;
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
            for node in ast {
                let result = evaluator.eval_statement(node);

                if evaluator.die || result.is_err() {
                    if let Err(error) = result {
                        evaluator.output = error.get_message(input);
                    }

                    break;
                }
            }

            // TODO: return a value from the file
            Ok(PhpValue::new_null())
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

/// Checks if a PHP value matches a type.
///
/// If it doesn't, it returns the expected type.
pub fn php_value_matches_argument_type(
    r#type: &PhpArgumentType,
    php_value: &PhpValue,
    _line: usize,
) -> Result<(), String> {
    match r#type {
        PhpArgumentType::Nullable(r#type) => {
            if php_value.is_null() {
                return Ok(());
            }

            php_value_matches_argument_type(r#type, php_value, _line)
        }
        PhpArgumentType::Union(types) => {
            let matches_any = types
                .iter()
                .any(|ty| php_value_matches_argument_type(ty, php_value, _line).is_ok());

            if !matches_any {
                return Err(types
                    .iter()
                    .map(|ty| ty.to_string())
                    .collect::<Vec<_>>()
                    .join("|"));
            }

            Ok(())
        }
        PhpArgumentType::Intersection(types) => {
            for ty in types {
                if php_value_matches_argument_type(ty, php_value, _line).is_err() {
                    return Err(types
                        .iter()
                        .map(|ty| ty.to_string())
                        .collect::<Vec<_>>()
                        .join("&"));
                }
            }

            Ok(())
        }
        PhpArgumentType::Null => {
            if !php_value.is_null() {
                return Err("null".to_string());
            }

            Ok(())
        }
        PhpArgumentType::True => {
            if !php_value.is_bool() {
                return Err("true".to_string());
            }

            let b = php_value.as_bool();

            if !b {
                return Err("true".to_string());
            }

            Ok(())
        }
        PhpArgumentType::False => {
            if !php_value.is_bool() {
                return Err("false".to_string());
            };

            let b = php_value.as_bool();

            if b {
                return Err("false".to_string());
            }

            Ok(())
        }
        PhpArgumentType::Float => {
            if !php_value.is_float() {
                return Err("float".to_string());
            }

            Ok(())
        }
        PhpArgumentType::Bool => {
            if !php_value.is_bool() {
                return Err("bool".to_string());
            }

            Ok(())
        }
        PhpArgumentType::Int => {
            if !php_value.is_int() {
                return Err("int".to_string());
            }

            Ok(())
        }
        PhpArgumentType::String => {
            if !php_value.is_string() {
                return Err("string".to_string());
            }

            Ok(())
        }
        PhpArgumentType::Array => todo!(),
        PhpArgumentType::Object => {
            if !php_value.is_object() {
                return Err("object".to_string());
            }

            Ok(())
        }
        PhpArgumentType::Mixed => Ok(()),
        PhpArgumentType::Callable => todo!(),
        PhpArgumentType::Iterable => todo!(),
        PhpArgumentType::StaticReference => unreachable!(),
        PhpArgumentType::SelfReference => todo!(),
        PhpArgumentType::ParentReference => todo!(),
        PhpArgumentType::Named(object_name) => {
            if !php_value.is_object() {
                return Err(get_string_from_bytes(&object_name.name));
            }

            let object = php_value.as_object();

            if !object_name.instance_of_object(&object) {
                return Err(object.get_name_as_string());
            }

            Ok(())
        }
    }
}

pub fn get_identifier_values(ident: Identifier) -> (Vec<u8>, Span) {
    match ident {
        Identifier::SimpleIdentifier(i) => (i.value.bytes, i.span),
        Identifier::DynamicIdentifier(_) => todo!(),
    }
}

/// Converts a string (vector of bytes) to a number.
pub fn string_as_number(string: &[u8]) -> u64 {
    string
        .iter()
        .fold(0u64, |acc, &byte| (acc << 8) | byte as u64)
}
