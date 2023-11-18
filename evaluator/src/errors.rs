use crate::php_value::PhpError;

pub fn expected_type_but_got(r#type: &str, given: String, line: usize) -> PhpError {
    PhpError {
        level: crate::php_value::ErrorLevel::Fatal,
        message: format!("Expected type '{}', '{}' given", r#type, given,),
        line,
    }
}

pub fn cannot_use_type_as_default_value_for_parameter_default_of_type(
    bad_type: String,
    parameter_name: String,
    r#type: String,
    line: usize,
) -> PhpError {
    PhpError {
        level: crate::php_value::ErrorLevel::Fatal,
        message: format!(
            "Cannot use {} as default value for parameter {} of type {}",
            bad_type, parameter_name, r#type
        ),
        line,
    }
}
