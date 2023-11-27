//! This file contains commonly used functions that return errors.
//! Only include functions that are intended to be used more than once.

use php_parser_rs::lexer::byte_string::ByteString;

use crate::{
    helpers::get_string_from_bytes,
    php_value::types::{PhpError, NULL},
};

pub fn expected_type_but_got(r#type: &str, given: String, line: usize) -> PhpError {
    PhpError {
        level: crate::php_value::types::ErrorLevel::Fatal,
        message: format!("Expected type '{}', '{}' given", r#type, given,),
        line,
    }
}

/// Returns an error when a type cannot be used as a default value for a property of a given type.
///
/// Note that the message is different for nullable types.
pub fn cannot_use_type_as_default_value_for_property_of_type(
    bad_type: String,
    class_name: &str,
    property_name: &str,
    expected_type: &str,
    line: usize,
) -> PhpError {
    if bad_type == NULL {
        let err = format!(
			"Default value for property of type {} may not be null. Use the nullable type ?{} to allow null default value",
			expected_type,
			expected_type
		);

        return PhpError {
            level: crate::php_value::types::ErrorLevel::Fatal,
            message: err,
            line,
        };
    }

    PhpError {
        level: crate::php_value::types::ErrorLevel::Fatal,
        message: format!(
            "Cannot use {} as default value for property {}::{} of type {}",
            bad_type, class_name, property_name, expected_type
        ),
        line,
    }
}

pub fn cannot_redeclare_method(class_name: &str, method: ByteString, line: usize) -> PhpError {
    PhpError {
        level: crate::php_value::types::ErrorLevel::Fatal,
        message: format!(
            "Cannot redeclare {}::{}()",
            class_name,
            get_string_from_bytes(&method.bytes),
        ),
        line,
    }
}

pub fn cannot_redeclare_property(class_name: &str, property: ByteString, line: usize) -> PhpError {
    PhpError {
        level: crate::php_value::types::ErrorLevel::Fatal,
        message: format!(
            "Cannot redeclare {}::{}",
            class_name,
            get_string_from_bytes(&property.bytes),
        ),
        line,
    }
}
