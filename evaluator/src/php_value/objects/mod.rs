pub mod class;

use std::collections::HashMap;

use php_parser_rs::{
    lexer::token::Span,
    parser::ast::{
        attributes::AttributeGroup,
        functions::ReturnType,
        identifiers::SimpleIdentifier,
        modifiers::{ClassModifierGroup, MethodModifierGroup, VisibilityModifier},
    },
};

use crate::helpers::{
    extend_hashmap_without_overwrite, get_string_from_bytes, visibility_modifier_to_method_modifier,
};

use self::class::{
    PhpClass, PhpObjectConcreteConstructor, PhpObjectConcreteMethod, PhpObjectConstant,
    PhpObjectProperty,
};

use super::{
    error::{ErrorLevel, PhpError},
    macros::impl_utils_for_php_objects,
    primitive_data_types::PhpFunctionArgument,
};

impl_utils_for_php_objects!(PhpClass, PhpAbstractClass);

#[derive(Debug, Clone)]
pub enum PhpObject {
    Class(PhpClass),
    AbstractClass(PhpAbstractClass),
    Trait(PhpTrait),
}

pub enum PhpObjectType {
    /// Both abstract classes and normal classes.
    Class,
    Trait,
}

impl PhpObject {
    pub fn extend(&mut self, parent: &PhpObject) -> Result<(), PhpError> {
        match self {
            PhpObject::Class(class) => class.extend(parent),
            PhpObject::AbstractClass(class) => class.extend(parent),
            PhpObject::Trait(_) => unreachable!(),
        }
    }

    pub fn set_parent(&mut self, parent: Box<PhpObject>) {
        match self {
            PhpObject::Class(class) => class.parent = Some(parent),
            PhpObject::AbstractClass(class) => class.parent = Some(parent),
            PhpObject::Trait(_) => unreachable!(),
        }
    }

    pub fn get_name_as_string(&self) -> String {
        match self {
            PhpObject::Class(class) => class.name.to_string(),
            PhpObject::AbstractClass(class) => class.name.to_string(),
            PhpObject::Trait(trait_) => trait_.name.to_string(),
        }
    }

    pub fn get_parent(&self) -> Option<&PhpObject> {
        match self {
            PhpObject::Class(class) => class.parent.as_ref().map(|parent| parent.as_ref()),
            PhpObject::AbstractClass(class) => class.parent.as_ref().map(|parent| parent.as_ref()),
            PhpObject::Trait(_) => None,
        }
    }

    pub fn instance_of(&self, object: &PhpObject) -> bool {
        match self {
            PhpObject::Class(class) => class.instance_of(object),
            PhpObject::AbstractClass(class) => class.instance_of(object),
            PhpObject::Trait(_) => todo!(),
        }
    }

    pub fn get_name_as_bytes(&self) -> &[u8] {
        match self {
            PhpObject::Class(class) => &class.name.value.bytes,
            PhpObject::AbstractClass(class) => &class.name.value.bytes,
            PhpObject::Trait(trait_) => &trait_.name.value.bytes,
        }
    }

    pub fn get_name_span(&self) -> Span {
        match self {
            PhpObject::Class(class) => class.name.span,
            PhpObject::AbstractClass(class) => class.name.span,
            PhpObject::Trait(trait_) => trait_.name.span,
        }
    }

    pub fn get_name_as_box(&self) -> Box<[u8]> {
        match self {
            PhpObject::Class(class) => class.name.value.bytes.clone().into_boxed_slice(),
            PhpObject::AbstractClass(class) => class.name.value.bytes.clone().into_boxed_slice(),
            PhpObject::Trait(trait_) => trait_.name.value.bytes.clone().into_boxed_slice(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhpAbstractClass {
    pub name: SimpleIdentifier,
    pub modifiers: ClassModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub parent: Option<Box<PhpObject>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<PhpTrait>,
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
    pub parameters: Vec<PhpFunctionArgument>,
    pub return_type: Option<ReturnType>,
}

#[derive(Debug, Clone)]
pub struct PhpTrait {
    pub name: SimpleIdentifier,
    pub attributes: Vec<AttributeGroup>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<PhpTrait>,
    pub concrete_methods: HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    pub concrete_constructor: Option<PhpObjectConcreteConstructor>,
    pub abstract_methods: HashMap<Vec<u8>, PhpObjectAbstractMethod>,
    pub abstract_constructor: Option<PhpObjectAbstractMethod>,
}

impl PhpTrait {
    /// Sets an alias for the given method, deleting the previous key.
    pub fn set_alias(
        &mut self,
        key: &[u8],
        alias: &[u8],
        class_name: &str,
        line: usize,
        visibility: Option<&VisibilityModifier>,
    ) -> Result<(), PhpError> {
        if !self.concrete_methods.contains_key(key) && !self.abstract_methods.contains_key(key) {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "An alias ({}) was defined for method {} but this method does not exist",
                    get_string_from_bytes(alias),
                    get_string_from_bytes(key),
                ),
                line,
            });
        }

        if self.concrete_methods.contains_key(alias) || self.abstract_methods.contains_key(alias) {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
					"Trait method {}::{} has not been applied as {}::{}, because of collision with {}::{}",
					&self.name.value.to_string(),
					get_string_from_bytes(key),
					class_name,
					get_string_from_bytes(alias),
					&self.name.value.to_string(),
					get_string_from_bytes(alias),
				),
                line,
            });
        }

        if self.concrete_methods.contains_key(key) {
            let mut concrete_method = self.concrete_methods.remove(key).unwrap();

            if let Some(visibility) = visibility {
                concrete_method.modifiers.modifiers =
                    vec![visibility_modifier_to_method_modifier(visibility)];
            }

            self.concrete_methods
                .insert(alias.to_vec(), concrete_method);

            return Ok(());
        } else if self.abstract_methods.contains_key(key) {
            let mut abstract_method = self.abstract_methods.remove(key).unwrap();

            if let Some(visibility) = visibility {
                abstract_method.modifiers.modifiers =
                    vec![visibility_modifier_to_method_modifier(visibility)];
            }

            self.abstract_methods
                .insert(alias.to_vec(), abstract_method);

            return Ok(());
        }

        Err(PhpError {
            level: ErrorLevel::Fatal,
            message: format!(
                "An alias was defined for {}::{} but this method does not exist",
                &self.name.value.to_string(),
                get_string_from_bytes(key)
            ),
            line,
        })
    }

    /// Sets the visibility for the given method, overwriting the previous modifiers.
    pub fn set_visibility(
        &mut self,
        key: &[u8],
        visibility: &VisibilityModifier,
        line: usize,
        method_name: &SimpleIdentifier,
    ) -> Result<(), PhpError> {
        if !self.concrete_methods.contains_key(key) && !self.abstract_methods.contains_key(key) {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
					"The modifiers of the trait method {}() are changed, but this method does not exist. Error",
					method_name
				),
                line,
            });
        }

        if self.concrete_methods.contains_key(key) {
            let concrete_method = self.concrete_methods.get_mut(key).unwrap();

            concrete_method.modifiers.modifiers =
                vec![visibility_modifier_to_method_modifier(visibility)];

            return Ok(());
        } else if self.abstract_methods.contains_key(key) {
            let abstract_method = self.abstract_methods.get_mut(key).unwrap();

            abstract_method.modifiers.modifiers =
                vec![visibility_modifier_to_method_modifier(visibility)];

            return Ok(());
        }

        Ok(())
    }

    pub fn remove_method(&mut self, method_name: &[u8]) {
        self.concrete_methods.remove(method_name);
        self.abstract_methods.remove(method_name);
    }
}