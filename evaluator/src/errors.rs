use crate::php_value::PhpError;

pub fn expected_type_but_got(r#type: &str, given: String, line: usize) -> PhpError {
    PhpError {
        level: crate::php_value::ErrorLevel::Fatal,
        message: format!(
            "Expected type '{}', '{}' given",
			r#type,
            given,
        ),
        line,
    }
}
