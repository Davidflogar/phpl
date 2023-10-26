use php_parser_rs::{lexer::token::Span, parser::ast::variables::Variable};

pub fn get_variable_span(var: Variable) -> Span {
    match var {
        Variable::SimpleVariable(v) => v.span,
        Variable::VariableVariable(vv) => vv.span,
        Variable::BracedVariableVariable(bvv) => bvv.start,
    }
}
