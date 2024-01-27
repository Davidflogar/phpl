use std::{cell::RefCell, collections::HashMap, rc::Rc};

use php_parser_rs::parser::ast::{
    constant::ClassishConstant,
    functions::{AbstractConstructor, AbstractMethod, ConcreteConstructor, ConcreteMethod},
    modifiers::MethodModifier,
    properties::{Property, PropertyEntry},
    traits::{TraitUsage, TraitUsageAdaptation},
};

use crate::{
    errors::{
        abstract_method_has_not_been_applied_because_of_collision, cannot_redeclare_method,
        cannot_redeclare_property, cannot_use_default_value_for_parameter,
        method_has_not_been_applied_because_of_collision, redefinition_of_parameter,
    },
    evaluator::Evaluator,
    helpers::{
        callable::eval_function_parameter_list, get_string_from_bytes,
        object::property_has_valid_default_value, php_value_matches_argument_type,
    },
    php_value::{
        argument_type::PhpArgumentType,
        error::{ErrorLevel, PhpError},
        objects::{
            class::{
                ConstructorNormalParameter, ConstructorParameter, ConstructorPromotedProperty,
                PhpObjectConcreteConstructor, PhpObjectConcreteMethod, PhpObjectConstant,
                PhpObjectProperty,
            },
            PhpObject, PhpObjectAbstractMethod, PhpTrait,
        },
        primitive_data_types::PhpValue,
    },
};

pub fn constant(
    evaluator: &mut Evaluator,
    constant: ClassishConstant,
    class_name: &str,
    consts: &mut HashMap<Vec<u8>, PhpObjectConstant>,
) -> Result<(), PhpError> {
    for entry in constant.entries {
        if consts.contains_key(&entry.name.value.bytes) {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Cannot redefine class constant {}::{}",
                    class_name, entry.name.value
                ),
                line: entry.name.span.line,
            });
        }

        let attributes = constant.attributes.clone();
        let modifiers = constant.modifiers.clone();

        let expr_result = evaluator.eval_expression(entry.value)?;

        consts.insert(
            entry.name.value.bytes,
            PhpObjectConstant {
                attributes,
                modifiers,
                value: expr_result,
            },
        );
    }

    Ok(())
}

pub fn property(
    evaluator: &mut Evaluator,
    property: Property,
    class_name: &str,
    properties: &mut HashMap<Vec<u8>, PhpObjectProperty>,
) -> Result<(), PhpError> {
    for entry in property.entries {
        let attributes = property.attributes.clone();
        let modifiers = property.modifiers.clone();
        let r#type = property.r#type.clone();

        let php_argument_type = if let Some(ty) = r#type {
            Some(PhpArgumentType::from_type(&ty, &evaluator.scope())?)
        } else {
            None
        };

        match entry {
            PropertyEntry::Initialized {
                variable,
                equals,
                value,
            } => {
                if properties.contains_key(&variable.name.bytes) {
                    return Err(cannot_redeclare_property(
                        class_name,
                        variable.name,
                        variable.span.line,
                    ));
                }

                let expr_value = evaluator.eval_expression(value)?;

                property_has_valid_default_value(
                    php_argument_type.as_ref(),
                    &expr_value,
                    equals.line,
                    class_name,
                    variable.name.to_string().as_str(),
                )?;

                let property = PhpObjectProperty {
                    attributes,
                    modifiers,
                    r#type: php_argument_type,
                    value: Rc::new(RefCell::new(expr_value)),
                    initialized: true,
                };

                properties.insert(variable.name.bytes, property);
            }
            PropertyEntry::Uninitialized { variable } => {
                if properties.contains_key(&variable.name.bytes) {
                    return Err(cannot_redeclare_property(
                        class_name,
                        variable.name,
                        variable.span.line,
                    ));
                }

                let property = PhpObjectProperty {
                    attributes,
                    modifiers,
                    r#type: php_argument_type,
                    value: Rc::new(RefCell::new(PhpValue::Null)),
                    initialized: false,
                };

                properties.insert(variable.name.bytes, property);
            }
        }
    }

    Ok(())
}

pub fn abstract_method(
    evaluator: &mut Evaluator,
    method: AbstractMethod,
    class_name: &str,
    abstract_methods: &mut HashMap<Vec<u8>, PhpObjectAbstractMethod>,
    concrete_methods: &HashMap<Vec<u8>, PhpObjectConcreteMethod>,
) -> Result<(), PhpError> {
    if abstract_methods.contains_key(&method.name.value.bytes)
        || concrete_methods.contains_key(&method.name.value.bytes)
    {
        return Err(cannot_redeclare_method(
            class_name,
            method.name.value,
            method.name.span.line,
        ));
    }

    for modifier in &method.modifiers.modifiers {
        let MethodModifier::Private(span) = modifier else {
			continue;
		};

        return Err(PhpError {
            level: ErrorLevel::Fatal,
            message: format!(
                "Abstract function {}::{}() cannot be declared private",
                class_name, method.name.value,
            ),
            line: span.line,
        });
    }

    let method_args = eval_function_parameter_list(method.parameters, evaluator)?;

    let abstract_method = PhpObjectAbstractMethod {
        attributes: method.attributes,
        modifiers: method.modifiers,
        return_by_reference: method.ampersand.is_some(),
        parameters: method_args,
        return_type: method.return_type,
    };

    abstract_methods.insert(method.name.value.bytes, abstract_method);

    Ok(())
}

pub fn abstract_constructor(
    evaluator: &mut Evaluator,
    constructor: AbstractConstructor,
    class_name: &str,
    abstract_constructor: Option<PhpObjectAbstractMethod>,
) -> Result<PhpObjectAbstractMethod, PhpError> {
    if abstract_constructor.is_some() {
        return Err(cannot_redeclare_method(
            class_name,
            constructor.name.value,
            constructor.name.span.line,
        ));
    }

    let method_args = eval_function_parameter_list(constructor.parameters, evaluator)?;

    Ok(PhpObjectAbstractMethod {
        attributes: constructor.attributes,
        modifiers: constructor.modifiers,
        return_by_reference: constructor.ampersand.is_some(),
        parameters: method_args,
        return_type: None,
    })
}

pub fn concrete_method(
    evaluator: &mut Evaluator,
    method: ConcreteMethod,
    class_name: &str,
    methods: &mut HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    abstract_methods: &HashMap<Vec<u8>, PhpObjectAbstractMethod>,
) -> Result<(), PhpError> {
    if methods.contains_key(&method.name.value.bytes)
        || abstract_methods.contains_key(&method.name.value.bytes)
    {
        return Err(cannot_redeclare_method(
            class_name,
            method.name.value,
            method.name.span.line,
        ));
    }

    let method_args = eval_function_parameter_list(method.parameters, evaluator)?;

    methods.insert(
        method.name.value.bytes,
        PhpObjectConcreteMethod {
            attributes: method.attributes,
            modifiers: method.modifiers,
            return_by_reference: method.ampersand.is_some(),
            name_span: method.name.span,
            parameters: method_args,
            return_type: method.return_type,
            body: method.body,
        },
    );

    Ok(())
}

pub fn concrete_constructor(
    evaluator: &mut Evaluator,
    constructor: ConcreteConstructor,
    class_name: &str,
    class_constructor: Option<PhpObjectConcreteConstructor>,
    properties: &mut HashMap<Vec<u8>, PhpObjectProperty>,
) -> Result<PhpObjectConcreteConstructor, PhpError> {

    if class_constructor.is_some() {
        return Err(cannot_redeclare_method(
            class_name,
            constructor.name.value,
            constructor.name.span.line,
        ));
    }

    let mut args: Vec<ConstructorParameter> = vec![];

    for param in constructor.parameters.parameters {
        let default_value_expression = param.default;
        let data_type = param.data_type.clone();
        let mut default_value = None;

        // check if the argument has already been declared
        for arg in &args {
            if arg.get_name_as_bytes() == param.name.name.bytes {
                return Err(redefinition_of_parameter(
                    &param.name.name,
                    param.name.span.line,
                ));
            }
        }

        if let (Some(default), Some(r#type)) = (default_value_expression, data_type) {
            let php_value = evaluator.eval_expression(default)?;

            let matches = php_value_matches_argument_type(
                &PhpArgumentType::from_type(&r#type, &evaluator.scope())?,
                &php_value,
                param.name.span.line,
            );

            if matches.is_err() {
                return Err(cannot_use_default_value_for_parameter(
                    php_value.get_type_as_string(),
                    param.name.name.to_string(),
                    r#type.to_string(),
                    param.name.span.line,
                ));
            }

            default_value = Some(php_value);
        }

        if !param.modifiers.is_empty() {
            // it is a promoted property

            if properties.contains_key(&param.name.name.bytes) {
                return Err(cannot_redeclare_property(
                    class_name,
                    param.name.name,
                    param.name.span.line,
                ));
            }

            let data_type = if let Some(r#type) = param.data_type {
                Some(PhpArgumentType::from_type(&r#type, &evaluator.scope())?)
            } else {
                None
            };

            args.push(ConstructorParameter::PromotedProperty(
                ConstructorPromotedProperty {
                    attributes: param.attributes,
                    pass_by_reference: param.ampersand.is_some(),
                    name: param.name.name.bytes,
                    data_type,
                    default: default_value,
                    modifiers: param.modifiers,
                    is_variadic: false,
                },
            ));
        } else {
            let data_type = if let Some(r#type) = param.data_type {
                Some(PhpArgumentType::from_type(&r#type, &evaluator.scope())?)
            } else {
                None
            };

            args.push(ConstructorParameter::Normal(ConstructorNormalParameter {
                attributes: param.attributes,
                pass_by_reference: param.ampersand.is_some(),
                name: param.name.name.bytes,
                data_type,
                default: default_value,
                is_variadic: param.ellipsis.is_some(),
            }));
        }
    }

    Ok(PhpObjectConcreteConstructor {
        attributes: constructor.attributes,
        modifiers: constructor.modifiers,
        return_by_reference: constructor.ampersand.is_some(),
        name: constructor.name,
        parameters: args,
        body: constructor.body,
    })
}

pub fn trait_usage(
    evaluator: &mut Evaluator,
    trait_statement: TraitUsage,
    class_name: &str,
    used_traits: &mut HashMap<Vec<u8>, PhpTrait>,
) -> Result<Vec<(Vec<u8>, PhpError)>, PhpError> {
    for trait_ in trait_statement.traits {
        let trait_name = trait_.value.bytes;

        if used_traits.contains_key(&trait_name) {
            continue;
        }

        let object_option = evaluator.scope().get_object_cloned(&trait_name);

        let Some(object) = object_option else {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!("Trait \"{}\" not found", get_string_from_bytes(&trait_name)),
                line: trait_.span.line,
            });
        };

        let PhpObject::Trait(trait_object) = object else {
			return Err(PhpError {
				level: ErrorLevel::Fatal,
				message: format!("{} cannot use {} - it is not a trait", class_name, get_string_from_bytes(&trait_name)),
				line: trait_.span.line,
			});
		};

        used_traits.insert(trait_name, trait_object);
    }

    for adaptation in trait_statement.adaptations {
        match adaptation {
            TraitUsageAdaptation::Alias {
                r#trait,
                method,
                alias,
                visibility,
            } => {
                if let Some(trait_name) = r#trait {
                    let trait_object_option = used_traits.get_mut(&trait_name.value.bytes);

                    let Some(trait_object) = trait_object_option else {
						return Err(PhpError {
							level: ErrorLevel::Fatal,
							message: format!("Trait \"{}\" was not added to {}", trait_name.value, class_name),
							line: trait_name.span.line,
						});
					};

                    trait_object.set_alias(
                        &method.value,
                        &alias.value,
                        class_name,
                        alias.span.line,
                        visibility.as_ref(),
                    )?;
                } else {
                    let mut found_in = String::new();

                    for trait_object in used_traits.values_mut() {
                        if !trait_object
                            .concrete_methods
                            .contains_key(&method.value.bytes)
                            && !trait_object
                                .abstract_methods
                                .contains_key(&method.value.bytes)
                        {
                            continue;
                        }

                        if !found_in.is_empty() {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
									"An alias was defined for method {}(), which exists in both {} and {}. \
									Use {}::{} or {}::{} to resolve the ambiguity",
									method,
									found_in,
									trait_object.name,
									found_in,
									method,
									trait_object.name,
									method,
								),
                                line: alias.span.line,
                            });
                        }

                        found_in = trait_object.name.value.to_string();

                        trait_object.set_alias(
                            &method.value,
                            &alias.value,
                            class_name,
                            alias.span.line,
                            visibility.as_ref(),
                        )?;
                    }
                }
            }
            TraitUsageAdaptation::Visibility {
                r#trait,
                method,
                visibility,
            } => {
                if let Some(trait_name) = r#trait {
                    let trait_object_option = used_traits.get_mut(&trait_name.value.bytes);

                    let Some(trait_object) = trait_object_option else {
						return Err(PhpError {
							level: ErrorLevel::Fatal,
							message: format!("Trait \"{}\" was not added to {}", trait_name.value, class_name),
							line: trait_name.span.line,
						});
					};

                    trait_object.set_visibility(
                        &method.value,
                        &visibility,
                        method.span.line,
                        &method,
                    )?;
                } else {
                    let mut found_in = String::new();

                    for trait_object in used_traits.values_mut() {
                        if !trait_object
                            .concrete_methods
                            .contains_key(&method.value.bytes)
                            && !trait_object
                                .abstract_methods
                                .contains_key(&method.value.bytes)
                        {
                            continue;
                        }

                        if !found_in.is_empty() {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
									"An alias was defined for method {}(), which exists in both {} and {}. \
									Use {}::{} or {}::{} to resolve the ambiguity",
									method,
									found_in,
									trait_object.name,
									found_in,
									method,
									trait_object.name,
									method,
								),
                                line: method.span.line,
                            });
                        }

                        found_in = trait_object.name.value.to_string();

                        trait_object.set_visibility(
                            &method.value,
                            &visibility,
                            method.span.line,
                            &method,
                        )?;
                    }
                }
            }
            TraitUsageAdaptation::Precedence {
                r#trait,
                method,
                insteadof,
            } => {
                if !used_traits.contains_key(&r#trait.value.bytes) {
                    return Err(PhpError {
                        level: ErrorLevel::Fatal,

                        message: format!(
                            "Trait \"{}\" was not added to {}",
                            r#trait.value, class_name
                        ),
                        line: r#trait.span.line,
                    });
                }

                for insteadof in insteadof {
                    if insteadof.value == r#trait.value {
                        return Err(PhpError {
							level: ErrorLevel::Fatal,
							message: format!(
								"Inconsistent insteadof definition. The method {} is to be used from {}, but {} is also on the exclude list",
								method,
								r#trait,
								r#trait,
							),
							line: insteadof.span.line,
						});
                    }

                    let Some(trait_object) = used_traits.get_mut(&insteadof.value.bytes) else {
						return Err(PhpError {
							level: ErrorLevel::Fatal,
							message: format!("Trait \"{}\" was not added to {}", insteadof, class_name),
							line: insteadof.span.line,
						});
					};

                    trait_object.remove_method(&method.value.bytes);
                }
            }
        }
    }

    // Find duplicated methods

    let mut concrete_methods_seen: Vec<(&[u8], &[u8], &PhpObjectConcreteMethod)> = vec![];

    let mut abstract_methods_seen: Vec<(&[u8], &[u8], &PhpObjectAbstractMethod)> = vec![];

    let mut duplicated_methods: Vec<(Vec<u8>, PhpError)> = vec![];

    for (trait_name, trait_object) in used_traits {
        for (method_name, method) in &trait_object.concrete_methods {
            if let Some(method) = concrete_methods_seen
                .iter()
                .find(|(name, _, _)| *name == method_name)
            {
                let error = method_has_not_been_applied_because_of_collision(
                    method_name,
                    method.1,
                    class_name,
                    trait_name,
                    trait_statement.r#use.line,
                );

                duplicated_methods.push((method_name.to_vec(), error));

                continue;
            }

            concrete_methods_seen.push((method_name, &trait_object.name.value.bytes, method));
        }

        for (method_name, method) in &trait_object.abstract_methods {
            if let Some(method) = abstract_methods_seen
                .iter()
                .find(|(name, _, _)| name == method_name)
            {
                let error = abstract_method_has_not_been_applied_because_of_collision(
                    method_name,
                    method.1,
                    class_name,
                    trait_name,
                    trait_statement.r#use.line,
                );

                duplicated_methods.push((method_name.to_vec(), error));

                continue;
            }

            abstract_methods_seen.push((method_name, &trait_object.name.value.bytes, method));
        }
    }

    Ok(duplicated_methods)
}
