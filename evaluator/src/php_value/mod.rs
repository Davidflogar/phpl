pub mod argument_type;
pub mod error;
pub mod objects;
pub mod primitive_data_types;

mod macros {
    macro_rules! impl_utils_for_php_objects {
		($($name:ident),*) => {
			$(
				impl $name {
					/// Extends the current object with the given object.
					pub fn extend(&mut self, parent_object: &PhpObject) -> Result<(), PhpError> {
						match parent_object {
							PhpObject::Class(parent) => {
								if parent.modifiers.has_final() {
									return Err(PhpError {
										level: ErrorLevel::Fatal,
										message: format!(
											"Class {} cannot extend final class {}",
											get_string_from_bytes(&self.name.value),
											get_string_from_bytes(&parent.name.value)
										),
										line: self.name.span.line,
									});
								}

								// get the properties and constants of the parent and add them to the current object
								extend_hashmap_without_overwrite(&mut self.properties, parent.properties.clone());
								extend_hashmap_without_overwrite(&mut self.consts, parent.consts.clone());
								extend_hashmap_without_overwrite(&mut self.methods, parent.methods.clone());

								Ok(())
							}
							PhpObject::AbstractClass(parent) => {
								if parent.modifiers.has_final() {
									return Err(PhpError {
										level: ErrorLevel::Fatal,
										message: format!(
											"Class {} cannot extend final class {}",
											get_string_from_bytes(&self.name.value),
											get_string_from_bytes(&parent.name.value)
										),
										line: self.name.span.line,
									});
								}

								// get the properties and constants of the parent and add them to the current object
								extend_hashmap_without_overwrite(&mut self.properties, parent.properties.clone());
								extend_hashmap_without_overwrite(&mut self.consts, parent.consts.clone());
								extend_hashmap_without_overwrite(&mut self.methods, parent.methods.clone());

								if !self.modifiers.has_abstract() {
									// validate the abstract methods/constructor
									let mut remaining_abstract_methods: Vec<String> = vec![];

									for (name, method) in &parent.abstract_methods {

										let current_method_option = self.methods.get(name);

										let Some(current_method) = current_method_option else {
											remaining_abstract_methods.push(get_string_from_bytes(&name));

											continue;
										};

										// check that the current method matches the abstract method
										let match_return_by_ref = method.return_by_reference == current_method.return_by_reference;
										let match_parameters = method.parameters == current_method.parameters;
										let match_return_type = method.return_type == current_method.return_type;

										if !match_return_by_ref || !match_parameters || !match_return_type {
											let format_parameter = |parameter: &PhpFunctionArgument| -> String {
												let data_type_as_string = if let Some(r#type) = &parameter.data_type {
													format!("{} ", r#type.to_string())
												} else {
													String::new()
												};

												format!(
													"{}{}{}",
													data_type_as_string,
													if parameter.is_variadic {"..."} else {""},
													get_string_from_bytes(&parameter.name.name),
												)
											};

											return Err(PhpError {
												level: ErrorLevel::Fatal,
												message: format!(
													"Declaration of {}::{}() must be compatible with {}{}::{}({}){}",
													get_string_from_bytes(&self.name.value),
													get_string_from_bytes(&name),
													if method.return_by_reference {"&"} else {""},
													get_string_from_bytes(&parent.name.value),
													get_string_from_bytes(&name),
													method.parameters
														.iter()
														.map(|parameter| format_parameter(parameter))
														.collect::<Vec<String>>()
														.join(", "),
													if let Some(r#type) = &method.return_type {
														format!(": {}", r#type.data_type)
													} else {
														String::new()
													}
												),
												line: current_method.name_span.line,
											});
										}
									}

									if !remaining_abstract_methods.is_empty() {
										return Err(PhpError {
											level: ErrorLevel::Fatal,
											message: format!(
												"Class {} contains {} abstract method and must therefore be declared abstract \
												or implement the remaining methods ({})",
												self.name,
												remaining_abstract_methods.len(),
												remaining_abstract_methods
													.iter()
													.map(|element| format!(
														"{}::{}",
														get_string_from_bytes(&parent.name.value), element)
													)
													.collect::<Vec<String>>()
													.join(", "),
											),
											line: self.name.span.line,
										})
									}
								}

								Ok(())
							}
							PhpObject::Trait(trait_) => Err(PhpError {
								level: ErrorLevel::Fatal,
								message: format!("Class {} cannot extend trait {}", self.name, trait_.name),
								line: trait_.name.span.line,
							})
						}
					}

					/// Checks if the given object is an instance of the current object.
					pub fn instance_of(&self, object: &PhpObject) -> bool {
						if object.get_name() == self.name.to_string() {
							return true;
						}

						if let Some(parent) = object.get_parent() {
							return self.instance_of(&parent);
						}

						false
					}
				}
			)*
		}
	}

    pub(crate) use impl_utils_for_php_objects;
}
