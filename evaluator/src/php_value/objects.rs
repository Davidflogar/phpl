use std::collections::HashMap;

use php_parser_rs::parser::ast::{
    attributes::AttributeGroup,
    data_type::Type,
    functions::{MethodBody, ReturnType},
    identifiers::SimpleIdentifier,
    modifiers::{
        ClassModifierGroup, ConstantModifierGroup, MethodModifierGroup,
        PromotedPropertyModifierGroup, PropertyModifierGroup,
    },
};

use crate::helpers::get_string_from_bytes;

use super::macros::impl_extend_for_php_objects;

impl_extend_for_php_objects!(PhpClass, PhpAbstractClass);

use super::primitive_data_types::{CallableArgument, ErrorLevel, PhpError, PhpValue};

#[derive(Debug, Clone)]
pub enum PhpObject {
    Class(PhpClass),
    AbstractClass(PhpAbstractClass),
}

impl PhpObject {
    pub fn extend(&mut self, parent: &PhpClass) -> Option<PhpError> {
        match self {
            PhpObject::Class(class) => class.extend(parent),
            PhpObject::AbstractClass(class) => class.extend(parent),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhpClass {
    pub name: SimpleIdentifier,
    pub modifiers: ClassModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub parent: Option<Box<PhpClass>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<SimpleIdentifier>,
    pub concrete_methods: HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    pub concrete_constructor: Option<PhpObjectConcreteConstructor>,
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

#[derive(Debug, Clone)]
pub struct PhpObjectConcreteMethod {
    pub attributes: Vec<AttributeGroup>,
    pub modifiers: MethodModifierGroup,
    pub return_by_reference: bool,
    pub name: SimpleIdentifier,
    pub parameters: Vec<CallableArgument>,
    pub return_type: Option<ReturnType>,
    pub body: MethodBody,
}

#[derive(Debug, Clone)]
pub struct PhpObjectConcreteConstructor {
    pub attributes: Vec<AttributeGroup>,
    pub modifiers: MethodModifierGroup,
    pub return_by_reference: bool,
    pub name: SimpleIdentifier,
    pub parameters: Vec<ConstructorParameter>,
    pub return_type: Option<ReturnType>,
    pub body: MethodBody,
}

#[derive(Debug, Clone)]
pub enum ConstructorParameter {
    PromotedProperty {
		attributes: Vec<AttributeGroup>,
		pass_by_reference: bool,
		name: Vec<u8>,
		data_type: Option<Type>,
		default: Option<PhpValue>,
		modifiers: PromotedPropertyModifierGroup,
    },
    Normal {
		attributes: Vec<AttributeGroup>,
		pass_by_reference: bool,
		name: Vec<u8>,
		data_type: Option<Type>,
		ellipsis: bool,
		default: Option<PhpValue>,
    },
}

#[derive(Debug, Clone)]
pub struct PhpAbstractClass {
    pub name: SimpleIdentifier,
    pub modifiers: ClassModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub parent: Option<Box<PhpClass>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<SimpleIdentifier>,
    pub abstract_methods: HashMap<Vec<u8>, PhpObjectAbstractMethod>,
    pub abstract_constructor: Option<PhpObjectAbstractMethod>,
    pub concrete_methods: HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    pub concrete_constructor: Option<PhpObjectConcreteConstructor>,
}

#[derive(Debug, Clone)]
pub struct PhpObjectAbstractMethod {
    pub attributes: Vec<AttributeGroup>,
    pub modifiers: MethodModifierGroup,
    pub return_by_reference: bool,
    pub name: SimpleIdentifier,
    pub parameters: Vec<CallableArgument>,
    pub return_type: Option<ReturnType>,
}

impl PhpObject {
    /// Returns the class if the object is a class, otherwise returns None.
    pub fn into_class(self) -> Option<PhpClass> {
        if let PhpObject::Class(class) = self {
            return Some(class);
        }

        None
    }
}

impl PhpClass {

    pub fn instance_of(self, object: PhpValue) -> Result<bool, PhpError> {
        if let PhpValue::Object(object) = object {
            let PhpObject::Class(object) = object else {
				return Err(PhpError {
					level: ErrorLevel::Fatal,
					message: "Left side of instanceof must be an object".to_string(),
					line: 0,
				});
			};

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
}
