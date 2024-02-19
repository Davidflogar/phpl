use std::collections::HashMap;

use php_parser_rs::parser::ast::traits::{TraitMember, TraitStatement};

use crate::{
    errors::cannot_redeclare_object,
    evaluator::Evaluator,
    php_data_types::{
        error::PhpError,
        objects::{
            class::PhpObjectConcreteConstructor, PhpObject, PhpObjectAbstractMethod, PhpObjectType,
            PhpTrait,
        },
        primitive_data_types::PhpValue,
    },
};

use super::objects;

pub fn statement(
    evaluator: &mut Evaluator,
    statement: TraitStatement,
) -> Result<PhpValue, PhpError> {
    if evaluator.scope().object_exists(&statement.name.value) {
        return Err(cannot_redeclare_object(
            &statement.name.value,
            statement.name.span.line,
            PhpObjectType::Trait,
        ));
    }

    let class_name = statement.name.value.to_string();

    // get the properties, methods, and rest of the class body
    let mut properties = HashMap::new();
    let mut consts = HashMap::new();
    let mut abstract_methods = HashMap::new();
    let mut abstract_constructor: Option<PhpObjectAbstractMethod> = None;
    let mut concrete_methods = HashMap::new();
    let mut class_constructor: Option<PhpObjectConcreteConstructor> = None;

    let mut used_traits = HashMap::new();

    let mut duplicated_methods = vec![];

    for member in statement.body.members {
        match member {
            TraitMember::Constant(constant) => {
                objects::object_body::constant(evaluator, constant, &class_name, &mut consts)?
            }
            TraitMember::TraitUsage(trait_statement) => {
                duplicated_methods.extend(objects::object_body::trait_usage(
                    evaluator,
                    trait_statement,
                    &class_name,
                    &mut used_traits,
                )?);
            }
            TraitMember::Property(property) => {
                objects::object_body::property(evaluator, property, &class_name, &mut properties)?
            }
            TraitMember::AbstractMethod(method) => objects::object_body::abstract_method(
                evaluator,
                method,
                &class_name,
                &mut abstract_methods,
                &concrete_methods,
            )?,
            TraitMember::AbstractConstructor(constructor) => {
                abstract_constructor = Some(objects::object_body::abstract_constructor(
                    evaluator,
                    constructor,
                    &class_name,
                    abstract_constructor,
                )?)
            }
            TraitMember::ConcreteMethod(method) => objects::object_body::concrete_method(
                evaluator,
                method,
                &class_name,
                &mut concrete_methods,
                &abstract_methods,
            )?,
            TraitMember::ConcreteConstructor(constructor) => {
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
        if !concrete_methods.contains_key(&method) && !abstract_methods.contains_key(&method) {
            return Err(error);
        }
    }

    let traits: Vec<PhpTrait> = used_traits.into_values().collect();

    let new_object = PhpTrait {
        name: statement.name,
        attributes: statement.attributes,
        properties,
        consts,
        traits,
        concrete_methods,
        concrete_constructor: class_constructor,
        abstract_methods,
        abstract_constructor,
    };

    evaluator
        .scope()
        .new_object(PhpObject::Trait(new_object), PhpObjectType::Trait)?;

    Ok(PhpValue::Null)
}
