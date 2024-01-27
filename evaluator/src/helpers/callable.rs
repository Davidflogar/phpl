use php_parser_rs::parser::ast::functions::FunctionParameterList;

use crate::{
    errors::{self, cannot_use_default_value_for_parameter},
    evaluator::Evaluator,
    php_value::{
        argument_type::PhpArgumentType, error::PhpError, primitive_data_types::PhpFunctionArgument,
    },
};

use super::php_value_matches_argument_type;

pub fn eval_function_parameter_list(
    params: FunctionParameterList,
    evaluator: &mut Evaluator,
) -> Result<Vec<PhpFunctionArgument>, PhpError> {
    let mut callable_args = vec![];
    let mut declared_args = vec![];

    for arg in params {
        if declared_args.contains(&arg.name.name.bytes) {
            return Err(errors::redefinition_of_parameter(
                &arg.name.name,
                arg.name.span.line,
            ));
        }

        let mut default_value = None;

        if arg.default.is_some() && arg.data_type.is_some() {
            let result_of_default_value = evaluator.eval_expression(arg.default.unwrap())?;
            let argument_data_type = arg.data_type.as_ref().unwrap();

            let is_not_valid = php_value_matches_argument_type(
                &PhpArgumentType::from_type(argument_data_type, &evaluator.scope())?,
                &result_of_default_value,
                0,
            );

            if is_not_valid.is_err() {
                return Err(cannot_use_default_value_for_parameter(
                    result_of_default_value.get_type_as_string(),
                    arg.name.name.to_string(),
                    argument_data_type.to_string(),
                    argument_data_type.first_span().line,
                ));
            }

            default_value = Some(result_of_default_value);
        } else if arg.default.is_some() && arg.data_type.is_none() {
            default_value = Some(evaluator.eval_expression(arg.default.unwrap())?);
        }

        let mut data_type: Option<PhpArgumentType> = None;

        if let Some(arg_data_type) = arg.data_type {
            data_type = Some(PhpArgumentType::from_type(
                &arg_data_type,
                &evaluator.scope(),
            )?);
        };

        callable_args.push(PhpFunctionArgument {
            name: arg.name.name.clone(),
            data_type,
            default_value,
            is_variadic: arg.ellipsis.is_some(),
            pass_by_reference: arg.ampersand.is_some(),
        });

        declared_args.push(arg.name.name.bytes);
    }

    Ok(callable_args)
}
