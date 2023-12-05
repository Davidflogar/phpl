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

use crate::helpers::{extend_hashmap_without_overwrite, get_string_from_bytes};

use super::macros::impl_utils_for_php_objects;

impl_utils_for_php_objects!(PhpClass, PhpAbstractClass);

use super::primitive_data_types::{CallableArgument, ErrorLevel, PhpError, PhpValue};

#[derive(Debug, Clone)]
pub enum PhpObject {
    Class(PhpClass),
    AbstractClass(PhpAbstractClass),
}

impl PhpObject {
    pub fn extend(&mut self, parent: PhpObject) -> Option<PhpError> {
        match self {
            PhpObject::Class(class) => class.extend(parent),
            PhpObject::AbstractClass(class) => class.extend(parent),
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            PhpObject::Class(class) => class.name.to_string(),
            PhpObject::AbstractClass(class) => class.name.to_string(),
        }
    }

	pub fn get_parent(&self) -> Option<&Box<PhpObject>> {
		match self {
			PhpObject::Class(class) => class.parent.as_ref(),
			PhpObject::AbstractClass(class) => class.parent.as_ref(),
		}
	}

	pub fn instance_of(&self, object: &PhpObject) -> bool {
		match self {
			PhpObject::Class(class) => class.instance_of(object),
			PhpObject::AbstractClass(class) => class.instance_of(object),
		}
	}
}

#[derive(Debug, Clone)]
pub struct PhpClass {
    pub name: SimpleIdentifier,
    pub modifiers: ClassModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub parent: Option<Box<PhpObject>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<SimpleIdentifier>,
    pub methods: HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    pub constructor: Option<PhpObjectConcreteConstructor>,
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
    pub parent: Option<Box<PhpObject>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<SimpleIdentifier>,
    pub abstract_methods: HashMap<Vec<u8>, PhpObjectAbstractMethod>,
    pub abstract_constructor: Option<PhpObjectAbstractMethod>,
    pub methods: HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    pub constructor: Option<PhpObjectConcreteConstructor>,
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
