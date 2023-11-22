use std::collections::HashMap;

use php_parser_rs::parser::ast::{
    classes::{ClassMember, ClassStatement},
    properties::PropertyEntry,
};

use crate::{
    evaluator::Evaluator,
    helpers::{get_string_from_bytes, object::property_has_valid_default_value},
    php_value::{
        php_object::{PhpObject, PhpObjectConstant, PhpObjectProperty},
        value::{ErrorLevel, PhpError, PhpValue},
    },
};

pub fn statement(evaluator: &mut Evaluator, class: ClassStatement) -> Result<PhpValue, PhpError> {
    let mut new_object = PhpObject {
        name: class.clone().name,
        modifiers: class.modifiers.clone(),
        attributes: class.attributes.clone(),
        parent: None,
        properties: HashMap::new(),
        consts: HashMap::new(),
        traits: vec![],
    };

    // get the parent if any
    if class.extends.is_some() {
        let extends = class.extends.as_ref().unwrap();

        let parent_name = &extends.parent.value.bytes;

        let parent_class = evaluator.env.get_class(parent_name);

        if parent_class.is_none() {
            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "Class \"{}\" not found",
                    get_string_from_bytes(parent_name)
                ),
                line: extends.parent.span.line,
            });
        }

        let parent_object = parent_class.unwrap();

        let can_extends = new_object.extend(&parent_object);

		if let Some(error) = can_extends {
			return Err(error);
		}

        new_object.parent = Some(Box::new(parent_object));
    }

    // get the properties, methods, and rest of the class body
    let mut properties = HashMap::new();
    let mut consts = HashMap::new();

    for member in class.body.members {
        match member {
            ClassMember::Constant(constant) => {
                for entry in constant.entries {
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
            ClassMember::TraitUsage(trait_usage) => {
                println!("{:#?}", trait_usage);
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
            _ => todo!(),
        }
    }

    new_object.properties.extend(properties);
    new_object.consts.extend(consts);

    println!("{:#?}", new_object.properties);

    let class_error = evaluator
        .env
        .new_class(&class.name.value.bytes, new_object, class.name.span);

	if let Some(error) = class_error {
		return Err(error);
	}

    Ok(PhpValue::Null)
}
