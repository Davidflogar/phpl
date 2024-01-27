use std::collections::HashMap;

use php_parser_rs::parser::ast::classes::{ClassMember, ClassStatement};

use crate::{
    errors::cannot_redeclare_object,
    evaluator::Evaluator,
    php_value::{
        error::{ErrorLevel, PhpError},
        objects::{
            class::{PhpClass, PhpObjectConcreteConstructor},
            PhpAbstractClass, PhpObject, PhpObjectAbstractMethod, PhpObjectType, PhpTrait,
        },
        primitive_data_types::PhpValue,
    },
};

use super::objects;

pub fn statement(evaluator: &mut Evaluator, class: ClassStatement) -> Result<PhpValue, PhpError> {
    if evaluator.scope().object_exists(&class.name.value) {
        return Err(cannot_redeclare_object(
            &class.name.value,
            class.name.span.line,
            PhpObjectType::Class,
        ));
    }

    let mut parent = None;

    let class_name = class.name.value.to_string();

    // get the parent if any
    if let Some(extends) = class.extends {
        let parent_name = &extends.parent.value;

        let parent_class = evaluator.scope().get_object_cloned(parent_name);

        if parent_class.is_none() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!("Class \"{}\" not found", parent_name),
                line: extends.parent.span.line,
            });
        }

        let parent_object = parent_class.unwrap();

        parent = Some(Box::new(parent_object));
    }

    // get the properties, methods, and rest of the class body

    let mut properties = HashMap::new();
    let mut consts = HashMap::new();
    let mut abstract_methods = HashMap::new();
    let mut abstract_constructor: Option<PhpObjectAbstractMethod> = None;
    let mut methods = HashMap::new();
    let mut class_constructor: Option<PhpObjectConcreteConstructor> = None;

    let mut used_traits = HashMap::new();

    let mut duplicated_methods = vec![];

    for member in class.body.members {
        match member {
            ClassMember::Constant(constant) => {
                objects::object_body::constant(evaluator, constant, &class_name, &mut consts)?
            }
            ClassMember::TraitUsage(trait_statement) => {
                duplicated_methods.extend(objects::object_body::trait_usage(
                    evaluator,
                    trait_statement,
                    &class_name,
                    &mut used_traits,
                )?);
            }
            ClassMember::Property(property) => {
                objects::object_body::property(evaluator, property, &class_name, &mut properties)?
            }
            ClassMember::AbstractMethod(method) => objects::object_body::abstract_method(
                evaluator,
                method,
                &class_name,
                &mut abstract_methods,
                &methods,
            )?,
            ClassMember::AbstractConstructor(constructor) => {
                abstract_constructor = Some(objects::object_body::abstract_constructor(
                    evaluator,
                    constructor,
                    &class_name,
                    abstract_constructor,
                )?)
            }
            ClassMember::ConcreteMethod(method) => objects::object_body::concrete_method(
                evaluator,
                method,
                &class_name,
                &mut methods,
                &abstract_methods,
            )?,
            ClassMember::ConcreteConstructor(constructor) => {
                class_constructor = Some(objects::object_body::concrete_constructor(
                    evaluator,
                    constructor,
                    &class_name,
                    class_constructor,
                    &mut properties,
                )?)
            }
            _ => todo!(),
        }
    }

    for (method, error) in duplicated_methods {
        if !methods.contains_key(&method) && !abstract_methods.contains_key(&method) {
            return Err(error);
        }
    }

    let traits: Vec<PhpTrait> = used_traits.into_values().collect();

    // create the new object

    let has_abstract = class.modifiers.has_abstract();

    let mut new_object = if has_abstract {
        PhpObject::AbstractClass(PhpAbstractClass {
            name: class.name,
            modifiers: class.modifiers,
            attributes: class.attributes,
            parent: None,
            properties,
            consts,
            traits,
            abstract_methods,
            abstract_constructor,
            methods,
            constructor: class_constructor,
        })
    } else {
        PhpObject::Class(PhpClass {
            name: class.name,
            modifiers: class.modifiers,
            attributes: class.attributes,
            parent: None,
            properties,
            consts,
            traits,
            methods,
            constructor: class_constructor,
        })
    };

    if let Some(parent_object) = parent {
        new_object.extend(&parent_object)?;

        new_object.set_parent(parent_object);
    }

    evaluator
        .scope()
        .new_object(new_object, PhpObjectType::Class)?;

    Ok(PhpValue::Null)
}
