use std::collections::HashMap;

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
        string_as_number,
    },
    php_data_types::{
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
    consts: &mut HashMap<u64, PhpObjectConstant>,
) -> Result<(), PhpError> {
    for entry in constant.entries {
        let entry_as_number = string_as_number(&entry.name.value);

        if consts.contains_key(&entry_as_number) {
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
            entry_as_number,
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
    properties: &mut HashMap<u64, PhpObjectProperty>,
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
                let variable_name_as_number = string_as_number(&variable.name.bytes);

                if properties.contains_key(&variable_name_as_number) {
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
                    value: expr_value,
                    initialized: true,
                };

                properties.insert(variable_name_as_number, property);
            }
            PropertyEntry::Uninitialized { variable } => {
                let variable_name_as_number = string_as_number(&variable.name.bytes);

                if properties.contains_key(&variable_name_as_number) {
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
                    value: PhpValue::new_null(),
                    initialized: false,
                };

                properties.insert(variable_name_as_number, property);
            }
        }
    }

    Ok(())
}

pub fn abstract_method(
    evaluator: &mut Evaluator,
    method: AbstractMethod,
    class_name: &str,
    abstract_methods: &mut HashMap<u64, PhpObjectAbstractMethod>,
    concrete_methods: &HashMap<u64, PhpObjectConcreteMethod>,
) -> Result<(), PhpError> {
    let method_name_as_number = string_as_number(&method.name.value.bytes);

    if abstract_methods.contains_key(&method_name_as_number)
        || concrete_methods.contains_key(&method_name_as_number)
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
        name: method.name.value.bytes,
        attributes: method.attributes,
        modifiers: method.modifiers,
        return_by_reference: method.ampersand.is_some(),
        parameters: method_args,
        return_type: method.return_type,
    };

    abstract_methods.insert(method_name_as_number, abstract_method);

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
        name: constructor.name.value.bytes,
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
    methods: &mut HashMap<u64, PhpObjectConcreteMethod>,
    abstract_methods: &HashMap<u64, PhpObjectAbstractMethod>,
) -> Result<(), PhpError> {
    let method_name_as_number = string_as_number(&method.name.value.bytes);

    if methods.contains_key(&method_name_as_number)
        || abstract_methods.contains_key(&method_name_as_number)
    {
        return Err(cannot_redeclare_method(
            class_name,
            method.name.value,
            method.name.span.line,
        ));
    }

    let method_args = eval_function_parameter_list(method.parameters, evaluator)?;

    methods.insert(
        method_name_as_number,
        PhpObjectConcreteMethod {
            name: method.name,
            attributes: method.attributes,
            modifiers: method.modifiers,
            return_by_reference: method.ampersand.is_some(),
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
    properties: &mut HashMap<u64, PhpObjectProperty>,
) -> Result<PhpObjectConcreteConstructor, PhpError> {
    if class_constructor.is_some() {
        return Err(cannot_redeclare_method(
            class_name,
            constructor.name.value,
            constructor.name.span.line,
        ));
    }

    let mut args: Vec<ConstructorParameter> = vec![];

    for constructor_param in constructor.parameters.parameters {
        let default_value_expression = constructor_param.default;
        let data_type = constructor_param.data_type.clone();
        let mut default_value = None;

        // check if the argument has already been declared
        for arg in &args {
            if arg.get_name_as_bytes() == constructor_param.name.name.bytes {
                return Err(redefinition_of_parameter(
                    &constructor_param.name.name,
                    constructor_param.name.span.line,
                ));
            }
        }

        if let (Some(default), Some(r#type)) = (default_value_expression, data_type) {
            let php_value = evaluator.eval_expression(default)?;

            let matches = php_value_matches_argument_type(
                &PhpArgumentType::from_type(&r#type, &evaluator.scope())?,
                &php_value,
                constructor_param.name.span.line,
            );

            if matches.is_err() {
                return Err(cannot_use_default_value_for_parameter(
                    php_value.get_type_as_string(),
                    constructor_param.name.name.to_string(),
                    r#type.to_string(),
                    constructor_param.name.span.line,
                ));
            }

            default_value = Some(php_value);
        }

        if !constructor_param.modifiers.is_empty() {
            let constructor_param_name_as_number =
                string_as_number(&constructor_param.name.name.bytes);

            // it is a promoted property
            if properties.contains_key(&constructor_param_name_as_number) {
                return Err(cannot_redeclare_property(
                    class_name,
                    constructor_param.name.name,
                    constructor_param.name.span.line,
                ));
            }

            let data_type = if let Some(r#type) = constructor_param.data_type {
                Some(PhpArgumentType::from_type(&r#type, &evaluator.scope())?)
            } else {
                None
            };

            args.push(ConstructorParameter::PromotedProperty(
                ConstructorPromotedProperty {
                    attributes: constructor_param.attributes,
                    pass_by_reference: constructor_param.ampersand.is_some(),
                    name: constructor_param.name.name.bytes,
                    data_type,
                    default: default_value,
                    modifiers: constructor_param.modifiers,
                    is_variadic: false,
                },
            ));
        } else {
            let data_type = if let Some(r#type) = constructor_param.data_type {
                Some(PhpArgumentType::from_type(&r#type, &evaluator.scope())?)
            } else {
                None
            };

            args.push(ConstructorParameter::Normal(ConstructorNormalParameter {
                attributes: constructor_param.attributes,
                pass_by_reference: constructor_param.ampersand.is_some(),
                name: constructor_param.name.name.bytes,
                data_type,
                default: default_value,
                is_variadic: constructor_param.ellipsis.is_some(),
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
    used_traits: &mut HashMap<u64, PhpTrait>,
) -> Result<Vec<(u64, PhpError)>, PhpError> {
    for trait_ in trait_statement.traits {
        let trait_name_as_number = string_as_number(&trait_.value.bytes);
        let trait_name_as_bytes = trait_.value.bytes;

        if used_traits.contains_key(&trait_name_as_number) {
            continue;
        }

        let object_option = evaluator.scope().get_object_cloned(&trait_name_as_bytes);

        let Some(object) = object_option else {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Trait \"{}\" not found",
                    get_string_from_bytes(&trait_name_as_bytes)
                ),
                line: trait_.span.line,
            });
        };

        let PhpObject::Trait(trait_object) = object else {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "{} cannot use {} - it is not a trait",
                    class_name,
                    get_string_from_bytes(&trait_name_as_bytes)
                ),
                line: trait_.span.line,
            });
        };

        used_traits.insert(trait_name_as_number, trait_object);
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
                    let trait_object_option =
                        used_traits.get_mut(&string_as_number(&trait_name.value));

                    let Some(trait_object) = trait_object_option else {
                        return Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: format!(
                                "Trait \"{}\" was not added to {}",
                                trait_name.value, class_name
                            ),
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
                    let method_name_as_number = string_as_number(&method.value.bytes);
                    let mut found_in = String::new();

                    for trait_object in used_traits.values_mut() {
                        if !trait_object
                            .concrete_methods
                            .contains_key(&method_name_as_number)
                            && !trait_object
                                .abstract_methods
                                .contains_key(&method_name_as_number)
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
                    let trait_object_option =
                        used_traits.get_mut(&string_as_number(&trait_name.value));

                    let Some(trait_object) = trait_object_option else {
                        return Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: format!(
                                "Trait \"{}\" was not added to {}",
                                trait_name.value, class_name
                            ),
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
                    let method_name_as_number = string_as_number(&method.value.bytes);
                    let mut found_in = String::new();

                    for trait_object in used_traits.values_mut() {
                        if !trait_object
                            .concrete_methods
                            .contains_key(&method_name_as_number)
                            && !trait_object
                                .abstract_methods
                                .contains_key(&method_name_as_number)
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
                if !used_traits.contains_key(&string_as_number(&r#trait.value)) {
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

                    let Some(trait_object) =
                        used_traits.get_mut(&string_as_number(&insteadof.value))
                    else {
                        return Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: format!(
                                "Trait \"{}\" was not added to {}",
                                insteadof, class_name
                            ),
                            line: insteadof.span.line,
                        });
                    };

                    trait_object.remove_method(&method.value.bytes);
                }
            }
        }
    }

    // Find duplicated methods

    let mut concrete_methods_seen: HashMap<&u64, (&[u8], &[u8])> = HashMap::new();

    let mut abstract_methods_seen: HashMap<&u64, (&[u8], &[u8])> = HashMap::new();

    let mut duplicated_methods = vec![];

    for r#trait in used_traits.values() {
        for (method_name, method) in &r#trait.concrete_methods {
            if let Some((previous_method_name, previous_trait_name)) = concrete_methods_seen.insert(
                method_name,
                (&method.name.value.bytes, &r#trait.name.value.bytes),
            ) {
                let error = method_has_not_been_applied_because_of_collision(
                    previous_method_name,
                    previous_trait_name,
                    class_name,
                    &r#trait.name.value,
                    r#trait.name.span.line,
                );

                duplicated_methods.push((*method_name, error));
            }
        }

        for (method_name, method) in &r#trait.abstract_methods {
            if let Some((previous_method_name, previous_trait_name)) =
                abstract_methods_seen.insert(method_name, (&method.name, &r#trait.name.value.bytes))
            {
                let error = abstract_method_has_not_been_applied_because_of_collision(
                    previous_method_name,
                    previous_trait_name,
                    class_name,
                    &r#trait.name.value,
                    r#trait.name.span.line,
                );

                duplicated_methods.push((*method_name, error));
            }
        }
    }

    Ok(duplicated_methods)
}
