pub mod objects;
pub mod primitive_data_types;

mod macros {
	macro_rules! impl_extend_for_php_objects {
		($($name:ident),*) => {
			$(
				impl $name {
					/// Extends the current object with the given object.
					pub fn extend(&mut self, parent: &PhpClass) -> Option<PhpError> {
						if parent.modifiers.has_final() {
							return Some(PhpError {
								level: super::primitive_data_types::ErrorLevel::Fatal,
								message: format!(
									"Class {} cannot extend final class {}",
									get_string_from_bytes(&self.name.value.bytes),
									get_string_from_bytes(&parent.name.value.bytes)
								),
								line: parent.name.span.line,
							});
						}

						// get the properties and constants of the parent and add them to the current object
						self.properties.extend(parent.properties.clone());
						self.consts.extend(parent.consts.clone());

						None
					}
				}
			)*
		}
	}

	pub(crate) use impl_extend_for_php_objects;
}
