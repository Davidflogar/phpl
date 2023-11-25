//! This file contains commonly used functions that return warnings.
//! Only include functions that are intended to be used more than once.

use php_parser_rs::lexer::token::Span;

use crate::php_value::types::PhpError;

pub fn string_conversion_failed(ty: String, span: Span) -> PhpError {
    PhpError {
        level: crate::php_value::types::ErrorLevel::Warning,
        message: format!("{} to string conversion failed.", ty),
        line: span.line,
    }
}
