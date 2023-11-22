use std::collections::HashMap;

use php_parser_rs::parser::ast::{
    attributes::AttributeGroup,
    data_type::Type,
    identifiers::SimpleIdentifier,
    modifiers::{ClassModifierGroup, ConstantModifierGroup, PropertyModifierGroup},
};

use crate::helpers::helpers::get_string_from_bytes;

use super::php_value::{PhpError, PhpValue, ErrorLevel};

#[derive(Debug, Clone)]
pub struct PhpObject {
    pub name: SimpleIdentifier,
    pub modifiers: ClassModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub parent: Option<Box<PhpObject>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    // TODO: pub implements: Vec<SimpleIdentifier>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<SimpleIdentifier>,
    // TODO: pub variable property
    // TODO: abstract methods
    // TODO: abstract constructor
    // TODO: concrete method
    // TODO: concrete constructor
}

#[derive(Debug, Clone)]
pub struct PhpObjectProperty {
    pub modifiers: PropertyModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub r#type: Option<Type>,
    pub value: PhpValue,
    pub initialized: bool,
}

#[derive(Debug, Clone)]
pub struct PhpObjectConstant {
    pub modifiers: ConstantModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub value: PhpValue,
}

impl PhpObject {
    pub fn is_instance_of(self, object: PhpValue) -> Result<bool, PhpError> {
        if let PhpValue::Object(object) = object {
            if object.name == self.name {
                return Ok(true);
            }

            if self.parent.is_some() && self.parent.unwrap().name == object.name {
                return Ok(true);
            }

            Ok(false)
        } else {
            Err(PhpError {
                level: ErrorLevel::Fatal,
                message: "Right side of instanceof must be an object".to_string(),
                line: 0,
            })
        }
    }

    /// Extends the current object with the given object.
    pub fn extend(&mut self, parent: &PhpObject) -> Option<PhpError> {
        if parent.modifiers.has_final() {
            return Some(PhpError {
                level: super::php_value::ErrorLevel::Fatal,
                message: format!(
                    "Class {} cannot extend final class {}",
                    get_string_from_bytes(&self.name.value.bytes),
                    get_string_from_bytes(&parent.name.value.bytes)
                ),
                line: parent.name.span.line,
            });
        }

        // get the properties and constants of the parent and add them to the current object
        self.properties.extend(parent.properties.clone());
        self.consts.extend(parent.consts.clone());

        None
    }
}
