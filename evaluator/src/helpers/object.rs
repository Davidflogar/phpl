use php_parser_rs::parser::ast::data_type::Type;

use crate::{
    errors,
    php_value::primitive_data_types::{PhpError, PhpValue},
};

// Checks that a property has a valid default value.
pub fn property_has_valid_default_value(
    r#type: Option<&Type>,
    default: &PhpValue,
    line: usize,
    class_name: &str,
    property_name: &str,
) -> Option<PhpError> {
    r#type?;

    match r#type.unwrap() {
        Type::Named(_, _) => todo!(),
        Type::Nullable(_, r#type) => {
            if let PhpValue::Null = default {
                return None;
            }

            property_has_valid_default_value(Some(r#type), default, line, class_name, property_name)
        }
        Type::Union(types) => {
            let matches_any = types.iter().any(|ty| {
                property_has_valid_default_value(Some(ty), default, line, class_name, property_name)
                    .is_none()
            });

            if !matches_any {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        &types
                            .iter()
                            .map(|ty| ty.to_string())
                            .collect::<Vec<_>>()
                            .join("|"),
                        line,
                    ),
                );
            }

            None
        }
        Type::Intersection(types) => {
            for ty in types {
                if property_has_valid_default_value(
                    Some(ty),
                    default,
                    line,
                    class_name,
                    property_name,
                ).is_some() {
                    return Some(
                        errors::cannot_use_type_as_default_value_for_property_of_type(
                            default.get_type_as_string(),
                            class_name,
                            property_name,
                            &types
                                .iter()
                                .map(|ty| ty.to_string())
                                .collect::<Vec<_>>()
                                .join("&"),
                            line,
                        ),
                    );
                }
            }

            None
        }
        Type::Void(_) => unreachable!(),
        Type::Null(_) => {
            if !matches!(default, PhpValue::Null) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "null",
                        line,
                    ),
                );
            }

            None
        }
        Type::True(_) => {
            let PhpValue::Bool(b) = *default else {
                return Some(errors::cannot_use_type_as_default_value_for_property_of_type(
                    default.get_type_as_string(),
					class_name,
					property_name,
                    "true",
                    line,
                ));
            };

            if !b {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "true",
                        line,
                    ),
                );
            }

            None
        }
        Type::False(_) => {
            let PhpValue::Bool(b) = *default else {
                return Some(errors::cannot_use_type_as_default_value_for_property_of_type(
                    default.get_type_as_string(),
					class_name,
					property_name,
                    "false",
                    line,
                ));
            };

            if b {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "false",
                        line,
                    ),
                );
            }

            None
        }
        Type::Never(_) => unreachable!(),
        Type::Float(_) => {
            if !matches!(default, PhpValue::Float(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "float",
                        line,
                    ),
                );
            }

            None
        }
        Type::Boolean(_) => {
            if !matches!(default, PhpValue::Bool(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "boolean",
                        line,
                    ),
                );
            }

            None
        }
        Type::Integer(_) => {
            if !matches!(default, PhpValue::Int(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "int",
                        line,
                    ),
                );
            }

            None
        }
        Type::String(_) => {
            if !matches!(default, PhpValue::String(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "string",
                        line,
                    ),
                );
            }

            None
        }
        Type::Array(_) => {
            if !matches!(default, PhpValue::Array(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "array",
                        line,
                    ),
                );
            }

            None
        }
        Type::Object(_) => {
            if !matches!(default, PhpValue::Object(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "object",
                        line,
                    ),
                );
            }

            None
        }
        Type::Mixed(_) => None,
        Type::Callable(_) => {
            if !matches!(default, PhpValue::Callable(_)) {
                return Some(
                    errors::cannot_use_type_as_default_value_for_property_of_type(
                        default.get_type_as_string(),
                        class_name,
                        property_name,
                        "callable",
                        line,
                    ),
                );
            }

            None
        }
        Type::Iterable(_) => todo!(),
        Type::StaticReference(_) => unreachable!(),
        Type::SelfReference(_) => todo!(),
        Type::ParentReference(_) => todo!(),
    }
}
