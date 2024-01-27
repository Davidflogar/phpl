use crate::{
    errors::cannot_use_type_as_default_value_for_property_of_type,
    php_value::{argument_type::PhpArgumentType, error::PhpError, primitive_data_types::PhpValue},
};

use super::php_value_matches_argument_type;

pub fn property_has_valid_default_value(
    r#type: Option<&PhpArgumentType>,
    php_value: &PhpValue,
    line: usize,
    class_name: &str,
    property_name: &str,
) -> Result<(), PhpError> {
    let Some(r#type) = r#type else {
		return Ok(());
	};

    let matches = php_value_matches_argument_type(r#type, php_value, line);

    if let Err(expected_type) = matches {
        return Err(cannot_use_type_as_default_value_for_property_of_type(
            php_value.get_type_as_string(),
            class_name,
            property_name,
            expected_type,
            line,
        ));
    }

    Ok(())
}
