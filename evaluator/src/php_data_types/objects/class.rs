use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    mem,
    rc::Rc,
};

use php_parser_rs::{
    lexer::token::Span,
    parser::ast::{
        arguments::{Argument, ArgumentList},
        attributes::AttributeGroup,
        functions::{MethodBody, ReturnType},
        identifiers::SimpleIdentifier,
        modifiers::{
            ClassModifierGroup, ConstantModifierGroup, MethodModifierGroup,
            PromotedPropertyModifier, PromotedPropertyModifierGroup, PropertyModifier,
            PropertyModifierGroup,
        },
        Expression, ReferenceExpression,
    },
};

use crate::{
    errors::{
        expected_type_but_got, only_arrays_and_traversables_can_be_unpacked,
        too_few_arguments_to_function,
    },
    evaluator::Evaluator,
    expressions::reference,
    helpers::{get_string_from_bytes, php_value_matches_argument_type},
    php_data_types::{
        argument_type::PhpArgumentType,
        error::{ErrorLevel, PhpError},
        macros::impl_validate_argument_for_struct,
        primitive_data_types::{PhpFunctionArgument, PhpValue},
    },
    scope::Scope,
};

use super::{PhpObject, PhpTrait};

#[derive(Debug, Clone)]
pub struct PhpClass {
    pub name: SimpleIdentifier,
    pub modifiers: ClassModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub parent: Option<Box<PhpObject>>,
    pub properties: HashMap<Vec<u8>, PhpObjectProperty>,
    pub consts: HashMap<Vec<u8>, PhpObjectConstant>,
    pub traits: Vec<PhpTrait>,
    pub methods: HashMap<Vec<u8>, PhpObjectConcreteMethod>,
    pub constructor: Option<PhpObjectConcreteConstructor>,
}

#[derive(Debug, Clone)]
pub struct PhpObjectProperty {
    pub modifiers: PropertyModifierGroup,
    pub attributes: Vec<AttributeGroup>,
    pub r#type: Option<PhpArgumentType>,
    pub value: Rc<RefCell<PhpValue>>,
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
    pub name_span: Span,
    pub parameters: Vec<PhpFunctionArgument>,
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

impl_validate_argument_for_struct!(ConstructorPromotedProperty, ConstructorNormalParameter);

#[derive(Debug, Clone)]
pub enum ConstructorParameter {
    PromotedProperty(ConstructorPromotedProperty),
    Normal(ConstructorNormalParameter),
}

impl ConstructorParameter {
    fn must_be_valid(
        &self,
        evaluator: &mut Evaluator,
        argument_type: Argument,
    ) -> Result<PhpValue, (Option<PhpError>, String)> {
        match self {
            ConstructorParameter::Normal(param) => param.must_be_valid(evaluator, argument_type),
            ConstructorParameter::PromotedProperty(param) => {
                param.must_be_valid(evaluator, argument_type)
            }
        }
    }

    pub fn get_name_as_bytes(&self) -> &[u8] {
        match self {
            ConstructorParameter::Normal(param) => &param.name,
            ConstructorParameter::PromotedProperty(param) => &param.name,
        }
    }

    fn get_name_as_vec(&self) -> Vec<u8> {
        self.get_name_as_bytes().to_vec()
    }
}

#[derive(Debug, Clone)]
pub struct ConstructorPromotedProperty {
    pub attributes: Vec<AttributeGroup>,
    pub pass_by_reference: bool,
    pub name: Vec<u8>,
    pub data_type: Option<PhpArgumentType>,
    pub default: Option<PhpValue>,
    pub modifiers: PromotedPropertyModifierGroup,

    /// Always false, but required for the macro impl_validate_argument_for_struct.
    pub is_variadic: bool,
}

#[derive(Debug, Clone)]
pub struct ConstructorNormalParameter {
    pub attributes: Vec<AttributeGroup>,
    pub pass_by_reference: bool,
    pub name: Vec<u8>,
    pub data_type: Option<PhpArgumentType>,
    pub is_variadic: bool,
    pub default: Option<PhpValue>,
}

impl PhpClass {
    /// This function is called when the class is instantiated.
    pub fn call_constructor(
        &mut self,
        evaluator: &mut Evaluator,
        arguments: Option<ArgumentList>,
        new: Span,
    ) -> Result<(), PhpError> {
        let Some(constructor) = self.constructor.as_mut() else {
			return Ok(());
		};

        let mut parameters_to_pass_to_the_constructor = HashMap::new();

        if !constructor.parameters.is_empty() {
            let constructor_parameters_len = constructor.parameters.len();
            let target_name = format!("{}::{}", self.name, constructor.name);

            let Some(constructor_call_arguments) = arguments else {

				return Err(too_few_arguments_to_function(
					target_name,
					0,
					constructor_parameters_len,
					new.line,
				))
			};

            let called_in_line = constructor_call_arguments.left_parenthesis.line;

            let mut required_arguments = VecDeque::new();

            for arg in &constructor.parameters {
                required_arguments.push_back(arg);
            }

            let constructor_call_paremeters_len = constructor_call_arguments.arguments.len();

            for (position, argument_type) in constructor_call_arguments.into_iter().enumerate() {
                match argument_type {
                    Argument::Positional(positional_argument) => {
                        if position > constructor_parameters_len - 1 {
                            break;
                        }

                        let constructor_arg = required_arguments.pop_front().unwrap();

                        // validate the argument
                        let validation_result = constructor_arg
                            .must_be_valid(evaluator, Argument::Positional(positional_argument));

                        if let Err((error, error_string)) = validation_result {
                            if error.is_none() {
                                let error = PhpError {
                                    level: ErrorLevel::Fatal,
                                    message: format!(
                                        "{}(): Argument #{} ({}): {}",
                                        target_name,
                                        position + 1,
                                        get_string_from_bytes(&constructor_arg.get_name_as_vec()),
                                        error_string
                                    ),
                                    line: called_in_line,
                                };

                                return Err(error);
                            }

                            return Err(error.unwrap());
                        }

                        parameters_to_pass_to_the_constructor.insert(
                            constructor_arg.get_name_as_vec(),
                            validation_result.unwrap(),
                        );
                    }
                    Argument::Named(named_argument) => {
                        let mut argument_name = named_argument.name.value.clone();

                        // add the $ at the beginning
                        // since the arguments inside required_arguments are saved with the $ at the beginning
                        argument_name.bytes.insert(0, b'$');

                        if parameters_to_pass_to_the_constructor.contains_key(&argument_name.bytes)
                        {
                            return Err(PhpError {
                                level: ErrorLevel::Fatal,
                                message: format!(
                                    "Named argument {} overwrites previous argument",
                                    get_string_from_bytes(&argument_name)
                                ),
                                line: named_argument.name.span.line,
                            });
                        }

                        let argument_position_some = required_arguments
                            .iter()
                            .position(|c| c.get_name_as_bytes() == argument_name.to_vec());

                        let Some(argument_position) = argument_position_some else {
							return Err(PhpError {
								level: ErrorLevel::Fatal,
								message: format!(
									"Unknown named argument {}",
									get_string_from_bytes(&argument_name)
								),
								line: named_argument.name.span.line,
							})
						};

                        let constructor_arg = required_arguments.remove(argument_position).unwrap();

                        // from here it is basically the same as working with a positional argument.
                        let validation_result = constructor_arg
                            .must_be_valid(evaluator, Argument::Named(named_argument));

                        if let Err((error, error_string)) = validation_result {
                            if error.is_none() {
                                let error = PhpError {
                                    level: ErrorLevel::Fatal,
                                    message: format!(
                                        "{}(): Argument #{} ({}): {}",
                                        target_name,
                                        position + 1,
                                        get_string_from_bytes(&constructor_arg.get_name_as_vec()),
                                        error_string
                                    ),
                                    line: called_in_line,
                                };

                                return Err(error);
                            }

                            return Err(error.unwrap());
                        }

                        parameters_to_pass_to_the_constructor.insert(
                            constructor_arg.get_name_as_vec(),
                            validation_result.unwrap(),
                        );
                    }
                }
            }

            let required_arguments_len = required_arguments.len();

            for required_arg in required_arguments {
                match required_arg {
                    ConstructorParameter::Normal(param) => {
                        let Some(ref default_value) = param.default else {
							return Err(too_few_arguments_to_function(
								target_name,
								constructor_call_paremeters_len,
								required_arguments_len,
								called_in_line,
							));
						};

                        parameters_to_pass_to_the_constructor
                            .insert(param.name.clone(), default_value.clone());
                    }
                    ConstructorParameter::PromotedProperty(promoted_property) => {
                        let Some(ref default_value) = promoted_property.default else {
							return Err(too_few_arguments_to_function(
								target_name,
								constructor_call_paremeters_len,
								required_arguments_len,
								called_in_line,
							));
						};

                        let property_value_as_reference =
                            Rc::new(RefCell::new(default_value.clone()));

                        // insert the parameter
                        parameters_to_pass_to_the_constructor.insert(
                            promoted_property.name.clone(),
                            PhpValue::Reference(Rc::clone(&property_value_as_reference)),
                        );

                        // insert the property
                        let mut property_modifiers = vec![];

                        for promoted_property_modifier in &promoted_property.modifiers.modifiers {
                            match promoted_property_modifier {
                                PromotedPropertyModifier::Public(span) => {
                                    property_modifiers.push(PropertyModifier::Public(*span));
                                }
                                PromotedPropertyModifier::Private(span) => {
                                    property_modifiers.push(PropertyModifier::Private(*span));
                                }
                                PromotedPropertyModifier::Protected(span) => {
                                    property_modifiers.push(PropertyModifier::Protected(*span));
                                }
                                PromotedPropertyModifier::Readonly(span) => {
                                    property_modifiers.push(PropertyModifier::Readonly(*span));
                                }
                            }
                        }

                        self.properties.insert(
                            promoted_property.name.clone(),
                            PhpObjectProperty {
                                modifiers: PropertyModifierGroup {
                                    modifiers: property_modifiers,
                                },
                                attributes: promoted_property.attributes.clone(),
                                r#type: promoted_property.data_type.clone(),
                                value: property_value_as_reference,
                                initialized: true,
                            },
                        );
                    }
                }
            }
        }

        let old_scope = Rc::clone(&evaluator.scope);

        let new_scope = Scope::new();

        evaluator.change_scope(Rc::new(RefCell::new(new_scope)));

        for new_var in parameters_to_pass_to_the_constructor {
            evaluator.scope().set_var_value(&new_var.0, new_var.1);
        }

        // execute the function
        let statements = mem::take(&mut constructor.body.statements);

        let mut error = None;

        for statement in statements {
            if let Err(err) = evaluator.eval_statement(statement) {
                error = Some(err);
                break;
            }
        }

        // change to the old environment
        evaluator.change_scope(old_scope);

        if let Some(err) = error {
            return Err(err);
        }

        Ok(())
    }
}
