use std::cell::RefCell;
use std::rc::Rc;
use std::{fs, str};

use php_parser_rs::parser::ast::operators::{
    BitwiseOperationExpression, ComparisonOperationExpression, LogicalOperationExpression,
};

use php_parser_rs::{
    lexer::token::Span,
    parser::ast::{
        literals::Literal,
        operators::{ArithmeticOperationExpression, AssignmentOperationExpression},
        variables::Variable,
        Expression, Statement,
    },
};

use crate::expressions::{function_call, new, reference};
use crate::helpers::callable::eval_function_parameter_list;
use crate::helpers::{get_identifier_values, get_string_from_bytes, parse_php_file};
use crate::php_data_types::error::{ErrorLevel, PhpError};
use crate::php_data_types::primitive_data_types::{PhpCallable, PhpIdentifier, PhpValue};
use crate::statements::{class, traits};
use crate::warnings;
use crate::{helpers::get_span_from_var, scope::Scope};

fn new_null() -> PhpValue {
    PhpValue::new_null()
}

pub struct Evaluator {
    pub output: String,

    /// Whether the PHP code is currently "open"
    php_open: bool,

    pub die: bool,

    pub scope: Rc<RefCell<Scope>>,

    pub warnings: Vec<PhpError>,

    pub included_files: Vec<String>,
    pub required_files: Vec<String>,
}

impl Evaluator {
    pub fn new(scope: Rc<RefCell<Scope>>) -> Evaluator {
        Evaluator {
            output: String::new(),
            php_open: false,
            die: false,
            scope,
            warnings: vec![],
            included_files: vec![],
            required_files: vec![],
        }
    }

    pub fn change_scope(&mut self, scope: Rc<RefCell<Scope>>) {
        self.scope = scope;
    }

    /// Appends the given output to the current evaluator's output.
    pub fn add_output(&mut self, output: &str) {
        self.output.push_str(output)
    }

    pub fn scope(&mut self) -> std::cell::RefMut<'_, Scope> {
        self.scope.borrow_mut()
    }

    pub fn eval_statement(&mut self, node: Statement) -> Result<(), PhpError> {
        match node {
            Statement::FullOpeningTag(_) => {
                self.php_open = true;

                Ok(())
            }
            Statement::ClosingTag(_) => {
                self.php_open = false;

                Ok(())
            }
            Statement::InlineHtml(html) => {
                self.add_output(&html.html.to_string());

                Ok(())
            }
            Statement::Expression(e) => {
                self.eval_expression(e.expression)?;

                Ok(())
            }
            Statement::Echo(echo) => {
                for expr in echo.values {
                    let expression_result = self.eval_expression(expr)?;

                    let expression_as_string = expression_result.printable();

                    if expression_as_string.is_none() {
                        self.warnings.push(warnings::string_conversion_failed(
                            expression_result.get_type_as_string(),
                            echo.echo,
                        ));

                        self.add_output(expression_result.get_type_as_string().as_str())
                    }

                    self.add_output(&expression_as_string.unwrap());
                }

                Ok(())
            }
            Statement::Function(func) => {
                let callable_args = eval_function_parameter_list(func.parameters, self)?;

                let php_callable = PhpCallable {
                    attributes: func.attributes,
                    return_by_reference: func.ampersand.is_some(),
                    parameters: callable_args,
                    return_type: func.return_type,
                    body: func.body.statements,
                    is_method: false,
                };

                self.scope().new_ident(
                    &func.name.value,
                    PhpIdentifier::Function(php_callable),
                    func.function,
                )?;

                Ok(())
            }
            Statement::Class(statement) => class::statement(self, statement),
            Statement::Trait(statement) => traits::statement(self, statement),
            _ => {
                println!("TODO: statement {:#?}\n", node);
                Ok(())
            }
        }
    }

    pub fn eval_expression(&mut self, expr: Expression) -> Result<PhpValue, PhpError> {
        match expr {
            Expression::Eval(_) => todo!(),
            Expression::Empty(ee) => {
                let arg = ee.argument.argument;

                let expression_result = self.eval_expression(arg.value)?;

                Ok(!expression_result)
            }
            Expression::Die(_) => {
                self.die();

                Ok(new_null())
            }
            Expression::Exit(_) => {
                self.die();

                Ok(new_null())
            }
            Expression::Isset(ie) => {
                for var in ie.variables {
                    let var_name = self.get_variable_name(var)?;

                    let scope = self.scope();

                    let var_exists = scope.get_var(&var_name);

                    let Some(value) = var_exists else {
                        return Ok(PhpValue::new_bool(false));
                    };

                    if value.is_null() {
                        return Ok(PhpValue::new_bool(false));
                    }
                }

                Ok(PhpValue::new_bool(true))
            }
            Expression::Unset(ue) => {
                for arg in ue.variables {
                    let var_name = self.get_variable_name(arg)?;

                    self.scope().delete_var(&var_name);
                }

                Ok(new_null())
            }
            Expression::Print(pe) => {
                let value_to_print = if let Some(value) = pe.value {
                    self.eval_expression(*value)?
                } else {
                    self.eval_expression(pe.argument.unwrap().argument.value)?
                };

                let value_as_string = value_to_print.printable();

                let Some(value) = value_as_string else {
                    self.warnings.push(warnings::string_conversion_failed(
                        value_to_print.get_type_as_string(),
                        pe.print,
                    ));

                    return Ok(PhpValue::new_string(Vec::new()));
                };

                self.add_output(value.as_str());

                Ok(new_null())
            }
            Expression::Literal(l) => match l {
                Literal::String(s) => Ok(PhpValue::new_string(s.value.bytes)),
                Literal::Integer(i) => {
                    let str_value = str::from_utf8(i.value.as_ref()).unwrap();

                    let int_value: i64 = str_value.parse().unwrap();

                    Ok(PhpValue::new_int(int_value))
                }
                Literal::Float(f) => {
                    let str_value = str::from_utf8(f.value.as_ref()).unwrap();

                    let float_value: f64 = str_value.parse().unwrap();

                    Ok(PhpValue::new_float(float_value))
                }
            },
            Expression::ArithmeticOperation(operation) => match operation {
                ArithmeticOperationExpression::Addition { left, plus, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(plus, left_value + right_value)
                }
                ArithmeticOperationExpression::Subtraction { left, minus, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(minus, left_value - right_value)
                }
                ArithmeticOperationExpression::Multiplication {
                    left,
                    asterisk,
                    right,
                } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(asterisk, left_value * right_value)
                }
                ArithmeticOperationExpression::Division { left, slash, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(slash, left_value / right_value)
                }
                ArithmeticOperationExpression::Modulo {
                    left,
                    percent,
                    right,
                } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(percent, left_value % right_value)
                }
                ArithmeticOperationExpression::Exponentiation { left, pow, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(pow, left_value.pow(right_value))
                }
                ArithmeticOperationExpression::Negative { right, minus } => {
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(minus, right_value * PhpValue::new_int(-1))
                }
                ArithmeticOperationExpression::Positive { right, plus } => {
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(plus, right_value * PhpValue::new_int(1))
                }
                ArithmeticOperationExpression::PreIncrement { .. } => todo!(),
                ArithmeticOperationExpression::PostIncrement { .. } => todo!(),
                ArithmeticOperationExpression::PreDecrement { .. } => todo!(),
                ArithmeticOperationExpression::PostDecrement { .. } => todo!(),
            },
            Expression::AssignmentOperation(operation) => match operation {
                AssignmentOperationExpression::Assign { left, right, .. } => {
                    let Expression::Variable(left_var) = *left else {
                        todo!()
                    };

                    let left_var_name = self.get_variable_name(left_var)?;

                    let right_value = self.eval_expression(*right)?;

                    self.scope().add_var_value(left_var_name, right_value);

                    Ok(new_null())
                }
                AssignmentOperationExpression::Addition {
                    left,
                    plus_equals,
                    right,
                } => self.change_var_value(*left, plus_equals, *right, "+"),
                AssignmentOperationExpression::Subtraction {
                    left,
                    minus_equals,
                    right,
                } => self.change_var_value(*left, minus_equals, *right, "-"),
                AssignmentOperationExpression::Multiplication {
                    left,
                    asterisk_equals,
                    right,
                } => self.change_var_value(*left, asterisk_equals, *right, "*"),
                AssignmentOperationExpression::Division {
                    left,
                    slash_equals,
                    right,
                } => self.change_var_value(*left, slash_equals, *right, "/"),
                AssignmentOperationExpression::Modulo {
                    left,
                    percent_equals,
                    right,
                } => self.change_var_value(*left, percent_equals, *right, "%"),
                AssignmentOperationExpression::Exponentiation {
                    left,
                    pow_equals,
                    right,
                } => self.change_var_value(*left, pow_equals, *right, "**"),
                AssignmentOperationExpression::Concat {
                    left,
                    dot_equals,
                    right,
                } => self.change_var_value(*left, dot_equals, *right, "."),
                AssignmentOperationExpression::BitwiseAnd {
                    left,
                    ampersand_equals,
                    right,
                } => self.change_var_value(*left, ampersand_equals, *right, "&"),
                AssignmentOperationExpression::BitwiseOr {
                    left,
                    pipe_equals,
                    right,
                } => self.change_var_value(*left, pipe_equals, *right, "|"),
                AssignmentOperationExpression::BitwiseXor {
                    left,
                    caret_equals,
                    right,
                } => self.change_var_value(*left, caret_equals, *right, "^"),
                AssignmentOperationExpression::LeftShift {
                    left,
                    left_shift_equals,
                    right,
                } => self.change_var_value(*left, left_shift_equals, *right, "<<"),
                AssignmentOperationExpression::RightShift {
                    left,
                    right_shift_equals,
                    right,
                } => self.change_var_value(*left, right_shift_equals, *right, ">>"),
                AssignmentOperationExpression::Coalesce {
                    left,
                    coalesce_equals,
                    right,
                } => self.change_var_value(*left, coalesce_equals, *right, "??"),
            },
            Expression::BitwiseOperation(operation) => match operation {
                BitwiseOperationExpression::And { left, and, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(and, left_value & right_value)
                }
                BitwiseOperationExpression::Or { left, or, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(or, left_value | right_value)
                }
                BitwiseOperationExpression::Xor { left, xor, right } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(xor, left_value ^ right_value)
                }
                BitwiseOperationExpression::LeftShift {
                    left,
                    left_shift,
                    right,
                } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(left_shift, left_value << right_value)
                }
                BitwiseOperationExpression::RightShift {
                    left,
                    right_shift,
                    right,
                } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    self.php_value_or_die(right_shift, left_value >> right_value)
                }
                BitwiseOperationExpression::Not { right, .. } => {
                    let right_value = self.eval_expression(*right)?;

                    Ok(!right_value)
                }
            },
            Expression::ComparisonOperation(operation) => match operation {
                ComparisonOperationExpression::Equal { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value == right_value))
                }
                ComparisonOperationExpression::Identical { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    if left_value.get_type_as_string() != right_value.get_type_as_string() {
                        return Ok(PhpValue::new_bool(false));
                    }

                    Ok(PhpValue::new_bool(left_value == right_value))
                }
                ComparisonOperationExpression::NotEqual { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value != right_value))
                }
                ComparisonOperationExpression::AngledNotEqual { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value != right_value))
                }
                ComparisonOperationExpression::NotIdentical { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    if left_value.get_type_as_string() != right_value.get_type_as_string() {
                        return Ok(PhpValue::new_bool(true));
                    }

                    Ok(PhpValue::new_bool(left_value != right_value))
                }
                ComparisonOperationExpression::LessThan { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value < right_value))
                }
                ComparisonOperationExpression::GreaterThan { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value > right_value))
                }
                ComparisonOperationExpression::LessThanOrEqual { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value <= right_value))
                }
                ComparisonOperationExpression::GreaterThanOrEqual { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(left_value >= right_value))
                }
                ComparisonOperationExpression::Spaceship { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    if left_value < right_value {
                        Ok(PhpValue::new_int(-1))
                    } else if left_value > right_value {
                        Ok(PhpValue::new_int(1))
                    } else {
                        Ok(PhpValue::new_int(0))
                    }
                }
            },
            Expression::LogicalOperation(operation) => match operation {
                LogicalOperationExpression::And { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(
                        left_value.true_in_php() && right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::Or { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(
                        left_value.true_in_php() || right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::Not { right, .. } => {
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(!right_value.true_in_php()))
                }
                LogicalOperationExpression::LogicalAnd { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(
                        left_value.true_in_php() && right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::LogicalOr { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(
                        left_value.true_in_php() || right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::LogicalXor { left, right, .. } => {
                    let left_value = self.eval_expression(*left)?;
                    let right_value = self.eval_expression(*right)?;

                    Ok(PhpValue::new_bool(
                        left_value.true_in_php() ^ right_value.true_in_php(),
                    ))
                }
            },
            Expression::Concat(expression) => {
                let left_value = self.eval_expression(*expression.left)?;
                let right_value = self.eval_expression(*expression.right)?;

                self.php_value_or_die(expression.dot, left_value.concat(right_value))
            }
            Expression::Instanceof(instanceof) => {
                let left_expr_value = self.eval_expression(*instanceof.left)?;

                if !left_expr_value.is_object() {
                    let error = format!(
                        "Left side of instanceof must be an object, got {}",
                        left_expr_value.get_type_as_string()
                    );

                    return Err(PhpError {
                        level: ErrorLevel::Fatal,
                        message: error,
                        line: instanceof.instanceof.line,
                    });
                };

                if let Expression::Identifier(ident) = *instanceof.right {
                    let (ident_value, ident_span) = get_identifier_values(ident);
                    let scope = self.scope();

                    let Some(object) = scope.get_object_by_ref(&ident_value) else {
                        return Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: format!(
                                "Undefined object {}",
                                get_string_from_bytes(&ident_value)
                            ),
                            line: ident_span.line,
                        });
                    };

                    let result = object.instance_of(&left_expr_value.as_object());

                    Ok(PhpValue::new_bool(result))
                } else {
                    let right_expr_value = self.eval_expression(*instanceof.right)?;

                    if !right_expr_value.is_object() {
                        return Ok(PhpValue::new_bool(false));
                    };

                    let left_object = left_expr_value.as_object();
                    let right_object = right_expr_value.as_object();

                    let result = left_object.instance_of(&right_object);

                    Ok(PhpValue::new_bool(result))
                }
            }
            Expression::Reference(reference) => match reference::expression(self, reference) {
                Ok(value) => Ok(value),
                Err((error, _)) => Err(error),
            },
            Expression::Parenthesized(parenthesized) => self.eval_expression(*parenthesized.expr),
            Expression::Identifier(ident) => {
                let (ident_value, ident_span) = get_identifier_values(ident);

                if let Some(ident) = self.scope().get_ident(&ident_value) {
                    return Ok(ident.as_php_value_cloned());
                }

                Err(PhpError {
                    level: ErrorLevel::Fatal,
                    message: format!(
                        "Undefined identifier {}",
                        get_string_from_bytes(&ident_value)
                    ),
                    line: ident_span.line,
                })
            }
            Expression::Variable(var) => self.get_var(var),
            Expression::Include(include) => {
                self.handle_include(*include.path, false, include.include)
            }
            Expression::IncludeOnce(include) => {
                self.handle_include(*include.path, true, include.include_once)
            }
            Expression::Require(require) => {
                self.handle_require(*require.path, false, require.require)
            }
            Expression::RequireOnce(require) => {
                self.handle_require(*require.path, true, require.require_once)
            }
            Expression::FunctionCall(call) => function_call::expression(self, call),
            Expression::FunctionClosureCreation(_) => todo!(),
            Expression::New(new) => new::expression(self, new),
            Expression::Bool(b) => Ok(PhpValue::new_bool(b.value)),
            _ => Ok(new_null()),
        }
    }

    /*
     * Private functions
     */

    fn die(&mut self) {
        self.die = true;
    }

    /// Check that `value` is PhpValue, if it is not it returns the error.
    ///
    /// It is used with arithmetic operations and logical operations.
    fn php_value_or_die(
        &mut self,
        span: Span,
        value: Result<PhpValue, PhpError>,
    ) -> Result<PhpValue, PhpError> {
        match value {
            Ok(value) => Ok(value),
            Err(mut error) => {
                error.line = span.line;

                Err(error)
            }
        }
    }

    pub fn get_variable_name(&mut self, variable: Variable) -> Result<Vec<u8>, PhpError> {
        match variable {
            Variable::SimpleVariable(sv) => Ok(sv.name.bytes),
            Variable::VariableVariable(vv) => {
                let value = self.get_variable_value(*vv.variable)?;

                if value.is_string() {
                    Ok(value.into_string().into_vec())
                } else {
                    let error = format!(
                        "Variable variable must be a string, got {}",
                        value.get_type_as_string(),
                    );

                    Err(PhpError {
                        level: ErrorLevel::Fatal,
                        message: error,
                        line: vv.span.line,
                    })
                }
            }
            Variable::BracedVariableVariable(bvv) => {
                let expr_value = self.eval_expression(*bvv.variable)?;

                let expr_as_string = expr_value.printable();

                if expr_as_string.is_none() {
                    self.warnings.push(warnings::string_conversion_failed(
                        expr_value.get_type_as_string(),
                        bvv.start,
                    ));

                    self.warnings.push(PhpError {
                        level: ErrorLevel::Warning,
                        message: format!("Undefined variable $ on line {}", bvv.start.line),
                        line: bvv.start.line,
                    });

                    return Ok(b"".to_vec());
                }

                let variable_name = expr_as_string.unwrap();

                Ok(variable_name.into())
            }
        }
    }

    fn get_variable_value(&mut self, variable: Variable) -> Result<PhpValue, PhpError> {
        match variable {
            Variable::SimpleVariable(sv) => {
                let var_name = &sv.name;

                let value = self.scope().get_var(var_name).cloned();

                if let Some(value) = value {
                    Ok(value)
                } else {
                    let warning = format!(
                        "Undefined variable {} on line {}",
                        get_string_from_bytes(var_name),
                        sv.span.line
                    );

                    self.warnings.push(PhpError {
                        level: ErrorLevel::Warning,
                        message: warning,
                        line: sv.span.line,
                    });

                    Ok(new_null())
                }
            }
            Variable::VariableVariable(vv) => self.get_var(*vv.variable),
            Variable::BracedVariableVariable(bvv) => {
                let expr_value = self.eval_expression(*bvv.variable)?;

                let expr_as_string = expr_value.printable();

                if expr_as_string.is_none() {
                    self.warnings.push(PhpError {
                        level: ErrorLevel::Warning,
                        message: format!(
                            "Braced variable variable must be a string, got {}",
                            expr_value.get_type_as_string(),
                        ),
                        line: bvv.start.line,
                    });

                    self.warnings.push(PhpError {
                        level: ErrorLevel::Warning,
                        message: "Undefined variable $".to_string(),
                        line: bvv.start.line,
                    });

                    return Ok(new_null());
                }

                let variable_name = expr_as_string.unwrap();

                if !self.scope().var_exists(variable_name.as_bytes()) {
                    self.warnings.push(PhpError {
                        level: ErrorLevel::Warning,
                        message: format!("Undefined variable $ on line {}", bvv.start.line),
                        line: bvv.start.line,
                    });

                    return Ok(new_null());
                }

                Ok(self
                    .scope()
                    .get_var(variable_name.as_bytes())
                    .unwrap()
                    .clone())
            }
        }
    }

    fn change_var_value(
        &mut self,
        left_expr: Expression,
        span: Span,
        right_expr: Expression,
        operation: &str,
    ) -> Result<PhpValue, PhpError> {
        let left = left_expr;
        let right = right_expr;

        let right_value = self.eval_expression(right)?;

        let Expression::Variable(var) = left else {
            todo!()
        };

        let var_name = self.get_variable_name(var)?;

        let current_var_value = self.scope().get_var(&var_name).cloned();

        if current_var_value.is_none() {
            let error = format!("Undefined variable {}", get_string_from_bytes(&var_name));

            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: error,
                line: span.line,
            });
        }

        let current_var_value = current_var_value.unwrap();

        let new_value = match operation {
            "+" => self.php_value_or_die(span, current_var_value + right_value),
            "-" => self.php_value_or_die(span, current_var_value - right_value),
            "*" => self.php_value_or_die(span, current_var_value * right_value),
            "/" => self.php_value_or_die(span, current_var_value / right_value),
            "%" => self.php_value_or_die(span, current_var_value % right_value),
            "**" => self.php_value_or_die(span, current_var_value.pow(right_value)),
            "." => self.php_value_or_die(span, current_var_value.concat(right_value)),
            "&" => self.php_value_or_die(span, current_var_value & right_value),
            "|" => self.php_value_or_die(span, current_var_value | right_value),
            "^" => self.php_value_or_die(span, current_var_value ^ right_value),
            "<<" => self.php_value_or_die(span, current_var_value << right_value),
            ">>" => self.php_value_or_die(span, current_var_value >> right_value),
            "??" => {
                if current_var_value.is_null() {
                    Ok(right_value)
                } else {
                    Ok(current_var_value)
                }
            }
            _ => Ok(new_null()),
        }?;

        self.scope().add_var_value(var_name, new_value);

        Ok(new_null())
    }

    /// Returns the value of the variable. If it does not exist, the warning is added and Null is returned.
    fn get_var(&mut self, variable: Variable) -> Result<PhpValue, PhpError> {
        let var_span = get_span_from_var(&variable);

        let var_name = self.get_variable_name(variable)?;

        let value = self.scope().get_var(&var_name).cloned();

        if let Some(value) = value {
            Ok(value)
        } else {
            let warning = format!("Undefined variable {}", get_string_from_bytes(&var_name));

            self.warnings.push(PhpError {
                level: ErrorLevel::Warning,
                message: warning,
                line: var_span.line,
            });

            Ok(new_null())
        }
    }

    fn handle_include(
        &mut self,
        path: Expression,
        once: bool,
        span: Span,
    ) -> Result<PhpValue, PhpError> {
        let path = self.eval_expression(path)?;

        let path_as_string = path.printable();

        if path_as_string.is_none() {
            self.warnings.push(warnings::string_conversion_failed(
                path.get_type_as_string(),
                span,
            ));
        }

        let real_relative_path = path_as_string.unwrap_or("".to_string());

        if real_relative_path.is_empty() {
            let error = "Path cannot be empty".to_string();

            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: error,
                line: span.line,
            });
        }

        let real_abs_path = fs::canonicalize(&real_relative_path);

        let fn_name = if once { "include_once" } else { "include" };

        if let Err(error) = real_abs_path {
            let error = PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "{}({}): Failed to open stream: {}",
                    fn_name, real_relative_path, error
                ),
                line: span.line,
            };

            self.warnings.push(error);

            return Ok(new_null());
        }

        let ok_abs_path = real_abs_path.unwrap();

        let path = ok_abs_path.to_str().unwrap();

        if once && self.included_files.iter().any(|i| i == path) {
            return Ok(PhpValue::new_bool(true));
        }

        let content = fs::read_to_string(path);

        if let Err(error) = content {
            let warning = PhpError {
                level: ErrorLevel::Warning,
                message: format!("{}({}): Failed to open stream: {}", fn_name, path, error),
                line: span.line,
            };

            self.warnings.push(warning);

            return Ok(new_null());
        }

        self.included_files.push(path.to_string());

        parse_php_file(self, path, &content.unwrap())
    }

    fn handle_require(
        &mut self,
        path: Expression,
        once: bool,
        span: Span,
    ) -> Result<PhpValue, PhpError> {
        let path = self.eval_expression(path)?;

        let path_as_string = path.printable();

        if path_as_string.is_none() {
            self.warnings.push(warnings::string_conversion_failed(
                path.get_type_as_string(),
                span,
            ));
        }

        let real_relative_path = path_as_string.unwrap_or("".to_string());

        if real_relative_path.is_empty() {
            let error = "Path cannot be empty".to_string();

            return Err(PhpError {
                level: ErrorLevel::Fatal,
                message: error,
                line: span.line,
            });
        }

        let real_abs_path = fs::canonicalize(&real_relative_path);

        let fn_name = if once { "require_once" } else { "require" };

        if let Err(error) = real_abs_path {
            let error = PhpError {
                level: ErrorLevel::Fatal,
                message: format!(
                    "{}({}): Failed to open stream: {}",
                    fn_name, real_relative_path, error
                ),
                line: span.line,
            };

            self.warnings.push(error);

            return Ok(new_null());
        }

        let ok_abs_path = real_abs_path.unwrap();

        let path = ok_abs_path.to_str().unwrap();

        if once && self.required_files.iter().any(|i| *i == path) {
            return Ok(PhpValue::new_bool(true));
        }

        let content = fs::read_to_string(path);

        if let Err(error) = content {
            let error = PhpError {
                level: ErrorLevel::Fatal,
                message: format!("{}({}): Failed to open stream: {}", fn_name, path, error),
                line: span.line,
            };

            self.warnings.push(error);

            return Ok(new_null());
        }

        self.required_files.push(path.to_string());

        parse_php_file(self, path, &content.unwrap())
    }
}
