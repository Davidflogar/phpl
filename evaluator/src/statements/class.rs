use std::collections::HashMap;

use php_parser_rs::parser::ast::{
    classes::{ClassMember, ClassStatement},
    properties::PropertyEntry,
};

use crate::{
    errors::{cannot_redeclare_method, cannot_redeclare_property},
    evaluator::Evaluator,
    helpers::{
        callable::parse_function_parameter_list, get_string_from_bytes,
        object::property_has_valid_default_value,
    },
    php_value::{
        objects::{
            PhpAbstractClass, PhpClass, PhpObject, PhpObjectAbstractMethod, PhpObjectConstant,
            PhpObjectProperty,
        },
        types::{ErrorLevel, PhpError, PhpValue},
    },
};

pub fn statement(evaluator: &mut Evaluator, class: ClassStatement) -> Result<PhpValue, PhpError> {
    let mut parent = None;
    let class_name = get_string_from_bytes(&class.name.value.bytes);

    // get the parent if any
    if let Some(extends) = class.extends {
        let parent_name = &extends.parent.value.bytes;

        let parent_class = evaluator.env.get_class(parent_name);

        if parent_class.is_none() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!("Class \"{}\" not found", get_string_from_bytes(parent_name)),
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

    // TODO: avoid so many calls to clone()
    for member in class.body.members {
        match member {
            ClassMember::Constant(constant) => {
                for entry in constant.entries {
                    if consts.contains_key(&entry.name.value.bytes) {
                        return Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: format!(
                                "Cannot redefine class constant {}::{}",
                                class_name,
                                get_string_from_bytes(&entry.name.value.bytes)
                            ),
                            line: entry.name.span.line,
                        });
                    }

                    let attributes = constant.attributes.clone();
                    let modifiers = constant.modifiers.clone();

                    consts.insert(
                        entry.name.value.bytes,
                        PhpObjectConstant {
                            attributes,
                            modifiers,
                            value: evaluator.eval_expression(&entry.value)?,
                        },
                    );
                }
            }
            ClassMember::Property(property) => {
                for entry in property.entries {
                    let attributes = property.attributes.clone();
                    let modifiers = property.modifiers.clone();
                    let r#type = property.r#type.clone();

                    match entry {
                        PropertyEntry::Initialized {
                            variable,
                            equals,
                            value,
                        } => {
                            if properties.contains_key(&variable.name.bytes) {
                                return Err(cannot_redeclare_property(
                                    &class_name,
                                    variable.name,
                                    variable.span.line,
                                ));
                            }

                            let expr_value = evaluator.eval_expression(&value)?;

                            let not_valid = property_has_valid_default_value(
                                r#type.as_ref(),
                                &expr_value,
                                equals.line,
                                get_string_from_bytes(&class.name.value.bytes).as_str(),
                                get_string_from_bytes(&variable.name.bytes).as_str(),
                            );

                            if let Some(error) = not_valid {
                                return Err(error);
                            }

                            let property = PhpObjectProperty {
                                attributes,
                                modifiers,
                                r#type,
                                value: expr_value,
                                initialized: true,
                            };

                            properties.insert(variable.name.bytes, property);
                        }
                        PropertyEntry::Uninitialized { variable } => {
                            if properties.contains_key(&variable.name.bytes) {
                                return Err(cannot_redeclare_property(
                                    &class_name,
                                    variable.name,
                                    variable.span.line,
                                ));
                            }

                            let property = PhpObjectProperty {
                                attributes,
                                modifiers,
                                r#type,
                                value: PhpValue::Null,
                                initialized: false,
                            };

                            properties.insert(variable.name.bytes, property);
                        }
                    }
                }
            }
            ClassMember::AbstractMethod(method) => {
                if abstract_methods.contains_key(&method.name.value.bytes) {
                    return Err(cannot_redeclare_method(
                        &class_name,
                        method.name.value,
                        method.name.span.line,
                    ));
                }

                let method_args = parse_function_parameter_list(method.parameters, evaluator)?;

                let abstract_method = PhpObjectAbstractMethod {
                    attributes: method.attributes,
                    modifiers: method.modifiers,
                    return_by_reference: method.ampersand.is_some(),
                    name: method.name.clone(),
                    parameters: method_args,
                    return_type: method.return_type,
                };

                abstract_methods.insert(method.name.value.bytes, abstract_method);
            }
            ClassMember::AbstractConstructor(constructor) => {
                if abstract_constructor.is_some() {
                    return Err(cannot_redeclare_method(
                        &class_name,
                        constructor.name.value,
                        constructor.name.span.line,
                    ));
                }

                let method_args = parse_function_parameter_list(constructor.parameters, evaluator)?;

                abstract_constructor = Some(PhpObjectAbstractMethod {
                    attributes: constructor.attributes,
                    modifiers: constructor.modifiers,
                    return_by_reference: constructor.ampersand.is_some(),
                    name: constructor.name,
                    parameters: method_args,
                    return_type: None,
                })
            }
            _ => todo!(),
        }
    }

    let has_abstract = class.modifiers.has_abstract();

    let mut new_object = if has_abstract {
        PhpObject::AbstractClass(PhpAbstractClass::new(
            class.name.clone(),
            properties,
            consts,
            class.modifiers,
            class.attributes,
            abstract_methods,
            abstract_constructor,
        ))
    } else {
        PhpObject::Class(PhpClass::new(
            class.name.clone(),
            properties,
            consts,
            class.modifiers,
            class.attributes,
        ))
    };

    if let Some(parent_object) = parent {
        let can_extends = new_object.extend(&parent_object);

        if let Some(error) = can_extends {
            return Err(error);
        }
    }

    println!("{:#?}", new_object);

    let class_error = evaluator
        .env
        .new_class(&class.name.value.bytes, new_object, class.name.span);

    if let Some(error) = class_error {
        return Err(error);
    }

    Ok(PhpValue::Null)
}
