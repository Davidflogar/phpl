use std::rc::Rc;
use std::{fs, str};

use php_parser_rs::parser::ast::identifiers::Identifier;
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

use crate::expressions::function_call;
use crate::helpers::callable::parse_function_parameter_list;
use crate::helpers::{get_string_from_bytes, parse_php_file};
use crate::php_value::primitive_data_types::PhpCallable;
use crate::statements::class;
use crate::warnings;
use crate::{
    environment::Environment,
    helpers::get_span_from_var,
    php_value::primitive_data_types::{ErrorLevel, PhpError, PhpValue},
};

const NULL: PhpValue = PhpValue::Null;

pub struct Evaluator<'a> {
    /// The output of the evaluated code
    pub output: String,

    /// Whether the PHP code is currently "open"
    php_open: bool,

    /// Whether the PHP code must die
    pub die: bool,

    pub env: &'a mut Environment,

    pub warnings: Vec<PhpError>,

    pub included_files: Vec<String>, // TODO: change the data type to a fixed and immutable string
    pub required_files: Vec<String>, // TODO: change the data type to a fixed and immutable string
}

impl<'a> Evaluator<'a> {
    pub fn new(env: &'a mut Environment) -> Evaluator<'a> {
        Evaluator {
            output: String::new(),
            php_open: false,
            die: false,
            env,
            warnings: vec![],
            included_files: vec![],
            required_files: vec![],
        }
    }

    /// Appends the given output to the current evaluator's output.
    pub fn add_output(&mut self, output: &str) {
        self.output.push_str(output)
    }

    pub fn eval_statement(&mut self, node: Statement) -> Result<PhpValue, PhpError> {
        match node {
            Statement::FullOpeningTag(_) => {
                self.php_open = true;

                Ok(NULL)
            }
            Statement::ClosingTag(_) => {
                self.php_open = false;

                Ok(NULL)
            }
            Statement::InlineHtml(html) => {
                self.add_output(&get_string_from_bytes(&html.html.bytes));

                Ok(NULL)
            }
            Statement::Expression(e) => {
                self.eval_expression(&e.expression)?;

                Ok(NULL)
            }
            Statement::Echo(echo) => {
                for expr in echo.values {
                    let expression_result = self.eval_expression(&expr)?;

                    let expression_as_string = expression_result.printable();

                    if expression_as_string.is_none() {
                        self.warnings.push(warnings::string_conversion_failed(
                            expression_result.get_type_as_string(),
                            echo.echo,
                        ));

                        self.add_output(expression_result.get_type_as_string().as_str())
                    }

                    self.add_output(expression_as_string.unwrap_or("".to_string()).as_str());
                }

                Ok(NULL)
            }
            Statement::Function(func) => {
                let callable_args = parse_function_parameter_list(func.parameters, self)?;

                let php_callable = PhpCallable {
                    attributes: func.attributes,
                    span: func.function,
                    return_by_reference: func.ampersand.is_some(),
                    name: func.name.value.clone(),
                    parameters: callable_args,
                    return_type: func.return_type,
                    body: func.body.statements,
                    is_method: false,
                };

                let set_identifier = self.env.new_ident(
                    &func.name.value.bytes,
                    PhpValue::Callable(php_callable),
                    func.function,
                );

                if let Some(err) = set_identifier {
                    return Err(err);
                }

                Ok(NULL)
            }
            Statement::Class(statement) => class::statement(self, statement),
            _ => {
                println!("TODO: statement {:#?}\n", node);
                Ok(NULL)
            }
        }
    }

    pub fn eval_expression(&mut self, expr: &Expression) -> Result<PhpValue, PhpError> {
        match expr {
            Expression::Eval(_) => todo!(),
            Expression::Empty(ee) => {
                let arg = ee.argument.argument.clone();

                let arg_as_php_value = self.eval_expression(&arg.value)?;

                let match_result = match arg_as_php_value {
                    PhpValue::Null => PhpValue::Bool(true),
                    PhpValue::Bool(b) => PhpValue::Bool(!b),
                    PhpValue::String(s) => {
                        let string = get_string_from_bytes(&s.bytes);

                        PhpValue::Bool(string.is_empty() || string == "0")
                    }
                    PhpValue::Float(f) => PhpValue::Bool(f == 0.0),
                    PhpValue::Int(i) => PhpValue::Bool(i == 0),
                    PhpValue::Array(a) => PhpValue::Bool(!a.is_empty()),
                    _ => PhpValue::Bool(false),
                };

                Ok(match_result)
            }
            Expression::Die(_) => {
                self.die();

                Ok(NULL)
            }
            Expression::Exit(_) => {
                self.die();

                Ok(NULL)
            }
            Expression::Isset(ie) => {
                let variables = &ie.variables;

                for var in variables {
                    let var_name = self.get_variable_name(var)?;

                    let var_exists = self.env.get_var(&var_name);

                    if var_exists.is_none() {
                        return Ok(PhpValue::Bool(false));
                    }
                }

                Ok(PhpValue::Bool(true))
            }
            Expression::Unset(ue) => {
                let args = &ue.variables;

                for arg in args {
                    let var_name = self.get_variable_name(arg)?;

                    self.env.delete_var(&var_name);
                }

                Ok(NULL)
            }
            Expression::Print(pe) => {
                if pe.value.is_some() {
                    let expr = *pe.value.clone().unwrap();

                    let value = self.eval_expression(&expr)?;

                    let value_as_string = value.printable();

                    if value_as_string.is_none() {
                        self.warnings.push(warnings::string_conversion_failed(
                            value.get_type_as_string(),
                            pe.print,
                        ));

                        return Ok(PhpValue::String(value.get_type_as_string().into()));
                    }

                    self.add_output(value_as_string.unwrap().as_str());
                } else if pe.argument.is_some() {
                    let arg = *pe.argument.clone().unwrap();

                    let value = self.eval_expression(&arg.argument.value)?;

                    let value_as_string = value.printable();

                    if value_as_string.is_none() {
                        self.warnings.push(warnings::string_conversion_failed(
                            value.get_type_as_string(),
                            pe.print,
                        ));

                        return Ok(PhpValue::String(value.get_type_as_string().into()));
                    }

                    self.add_output(value_as_string.unwrap().as_str());
                }

                Ok(NULL)
            }
            Expression::Literal(l) => match l {
                Literal::String(s) => Ok(PhpValue::String(s.value.clone())),
                Literal::Integer(i) => {
                    let str_value = str::from_utf8(i.value.as_ref()).unwrap();

                    let int_value: i32 = str_value.parse().unwrap();

                    Ok(PhpValue::Int(int_value))
                }
                Literal::Float(f) => {
                    let str_value = str::from_utf8(f.value.as_ref()).unwrap();

                    let float_value: f32 = str_value.parse().unwrap();

                    Ok(PhpValue::Float(float_value))
                }
            },
            Expression::ArithmeticOperation(operation) => match operation {
                ArithmeticOperationExpression::Addition { left, plus, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(plus, left_value + right_value)
                }
                ArithmeticOperationExpression::Subtraction { left, minus, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(minus, left_value - right_value)
                }
                ArithmeticOperationExpression::Multiplication {
                    left,
                    asterisk,
                    right,
                } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(asterisk, left_value * right_value)
                }
                ArithmeticOperationExpression::Division { left, slash, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(slash, left_value / right_value)
                }
                ArithmeticOperationExpression::Modulo {
                    left,
                    percent,
                    right,
                } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(percent, left_value % right_value)
                }
                ArithmeticOperationExpression::Exponentiation { left, pow, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(pow, left_value.pow(right_value))
                }
                ArithmeticOperationExpression::Negative { right, minus } => {
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(minus, right_value * PhpValue::Int(-1))
                }
                ArithmeticOperationExpression::Positive { right, plus } => {
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(plus, right_value * PhpValue::Int(1))
                }
                ArithmeticOperationExpression::PreIncrement { right, increment } => {
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(increment, PhpValue::Int(1) + right_value)
                }
                ArithmeticOperationExpression::PostIncrement { left, increment } => {
                    let left_value = self.eval_expression(left)?;

                    self.php_value_or_die(increment, left_value + PhpValue::Int(1))
                }
                ArithmeticOperationExpression::PreDecrement { right, decrement } => {
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(decrement, right_value - PhpValue::Int(1))
                }
                ArithmeticOperationExpression::PostDecrement { left, decrement } => {
                    let left_value = self.eval_expression(left)?;

                    self.php_value_or_die(decrement, left_value - PhpValue::Int(1))
                }
            },
            Expression::AssignmentOperation(operation) => match operation {
                AssignmentOperationExpression::Assign { left, right, .. } => {
                    let Expression::Variable(ref left_var) = **left else {
						todo!()
					};

                    let left_var_name = self.get_variable_name(left_var)?;

                    if let Expression::Reference(ref reference) = **right {
                        let Expression::Variable(ref right_var) = *reference.right else {
							todo!()
						};

                        let right_var_name = self.get_variable_name(right_var)?;

                        if !self.env.var_exists(&right_var_name) {
                            self.env.insert_var(&right_var_name, &NULL)
                        }

                        let right_value = self
                            .env
                            .get_var_with_rc(&right_var_name)
                            .unwrap()
                            .to_owned();

                        let cloned_right_value = Rc::clone(&right_value);

                        self.env.insert_var_rc(&left_var_name, cloned_right_value);

                        return Ok(right_value.borrow().clone());
                    } else {
                        let right_value = self.eval_expression(right)?;

                        if !self.env.var_exists(&left_var_name) {
                            self.env.insert_var(&left_var_name, &right_value);
                        } else {
                            let old_value = self.env.get_var_with_rc(&left_var_name).unwrap();

                            *old_value.borrow_mut() = right_value.clone()
                        }

                        Ok(right_value)
                    }
                }
                AssignmentOperationExpression::Addition {
                    left,
                    plus_equals,
                    right,
                } => self.change_var_value(left, plus_equals, right, "+"),
                AssignmentOperationExpression::Subtraction {
                    left,
                    minus_equals,
                    right,
                } => self.change_var_value(left, minus_equals, right, "-"),
                AssignmentOperationExpression::Multiplication {
                    left,
                    asterisk_equals,
                    right,
                } => self.change_var_value(left, asterisk_equals, right, "*"),
                AssignmentOperationExpression::Division {
                    left,
                    slash_equals,
                    right,
                } => self.change_var_value(left, slash_equals, right, "/"),
                AssignmentOperationExpression::Modulo {
                    left,
                    percent_equals,
                    right,
                } => self.change_var_value(left, percent_equals, right, "%"),
                AssignmentOperationExpression::Exponentiation {
                    left,
                    pow_equals,
                    right,
                } => self.change_var_value(left, pow_equals, right, "**"),
                AssignmentOperationExpression::Concat {
                    left,
                    dot_equals,
                    right,
                } => self.change_var_value(left, dot_equals, right, "."),
                AssignmentOperationExpression::BitwiseAnd {
                    left,
                    ampersand_equals,
                    right,
                } => self.change_var_value(left, ampersand_equals, right, "&"),
                AssignmentOperationExpression::BitwiseOr {
                    left,
                    pipe_equals,
                    right,
                } => self.change_var_value(left, pipe_equals, right, "|"),
                AssignmentOperationExpression::BitwiseXor {
                    left,
                    caret_equals,
                    right,
                } => self.change_var_value(left, caret_equals, right, "^"),
                AssignmentOperationExpression::LeftShift {
                    left,
                    left_shift_equals,
                    right,
                } => self.change_var_value(left, left_shift_equals, right, "<<"),
                AssignmentOperationExpression::RightShift {
                    left,
                    right_shift_equals,
                    right,
                } => self.change_var_value(left, right_shift_equals, right, ">>"),
                AssignmentOperationExpression::Coalesce {
                    left,
                    coalesce_equals,
                    right,
                } => self.change_var_value(left, coalesce_equals, right, "??"),
            },
            Expression::BitwiseOperation(operation) => match operation {
                BitwiseOperationExpression::And { left, and, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(and, left_value & right_value)
                }
                BitwiseOperationExpression::Or { left, or, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(or, left_value | right_value)
                }
                BitwiseOperationExpression::Xor { left, xor, right } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(xor, left_value ^ right_value)
                }
                BitwiseOperationExpression::LeftShift {
                    left,
                    left_shift,
                    right,
                } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(left_shift, left_value << right_value)
                }
                BitwiseOperationExpression::RightShift {
                    left,
                    right_shift,
                    right,
                } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(right_shift, left_value >> right_value)
                }
                BitwiseOperationExpression::Not { right, not } => {
                    let right_value = self.eval_expression(right)?;

                    self.php_value_or_die(not, !right_value)
                }
            },
            Expression::ComparisonOperation(operation) => match operation {
                ComparisonOperationExpression::Equal { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value == right_value))
                }
                ComparisonOperationExpression::Identical { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    if left_value.get_type_as_string() != right_value.get_type_as_string() {
                        return Ok(PhpValue::Bool(false));
                    }

                    Ok(PhpValue::Bool(left_value == right_value))
                }
                ComparisonOperationExpression::NotEqual { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value != right_value))
                }
                ComparisonOperationExpression::AngledNotEqual { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value != right_value))
                }
                ComparisonOperationExpression::NotIdentical { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    if left_value.get_type_as_string() != right_value.get_type_as_string() {
                        return Ok(PhpValue::Bool(true));
                    }

                    Ok(PhpValue::Bool(left_value != right_value))
                }
                ComparisonOperationExpression::LessThan { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value < right_value))
                }
                ComparisonOperationExpression::GreaterThan { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value > right_value))
                }
                ComparisonOperationExpression::LessThanOrEqual { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value <= right_value))
                }
                ComparisonOperationExpression::GreaterThanOrEqual { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(left_value >= right_value))
                }
                ComparisonOperationExpression::Spaceship { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    if left_value < right_value {
                        Ok(PhpValue::Int(-1))
                    } else if left_value > right_value {
                        Ok(PhpValue::Int(1))
                    } else {
                        Ok(PhpValue::Int(0))
                    }
                }
            },
            Expression::LogicalOperation(operation) => match operation {
                LogicalOperationExpression::And { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(
                        left_value.true_in_php() && right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::Or { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(
                        left_value.true_in_php() || right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::Not { right, .. } => {
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(!right_value.true_in_php()))
                }
                LogicalOperationExpression::LogicalAnd { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(
                        left_value.true_in_php() && right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::LogicalOr { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(
                        left_value.true_in_php() || right_value.true_in_php(),
                    ))
                }
                LogicalOperationExpression::LogicalXor { left, right, .. } => {
                    let left_value = self.eval_expression(left)?;
                    let right_value = self.eval_expression(right)?;

                    Ok(PhpValue::Bool(
                        left_value.true_in_php() ^ right_value.true_in_php(),
                    ))
                }
            },
            Expression::Concat(expression) => {
                let left_value = self.eval_expression(&expression.left)?;
                let right_value = self.eval_expression(&expression.right)?;

                self.php_value_or_die(&expression.dot, left_value.concat(right_value))
            }
            Expression::Instanceof(instanceof) => {
                let left_expr_value = self.eval_expression(&instanceof.left)?;

                let PhpValue::Object(left_object) = left_expr_value else {
					let error =
						format!(
							"Left side of instanceof must be an object, got {}",
							left_expr_value.get_type_as_string()
						);

					return Err(PhpError { level: ErrorLevel::Fatal, message: error, line: instanceof.instanceof.line });
				};

                let right_expr_value = self.eval_expression(&instanceof.right)?;

                let PhpValue::Object(right_object) = right_expr_value else {
					let error = format!(
							"Right side of instanceof must be an object, got {}",
							right_expr_value.get_type_as_string()
						);

					return Err(PhpError { level: ErrorLevel::Fatal, message: error, line: instanceof.instanceof.line });
				};

                Ok(PhpValue::Bool(left_object.instance_of(&right_object)))
            }
            Expression::Reference(_) => todo!(),
            Expression::Parenthesized(parenthesized) => self.eval_expression(&parenthesized.expr),
            Expression::ErrorSuppress(error_expression) => {
                let old_php_die = self.die;
                let old_warnings = self.warnings.clone();

                self.eval_expression(&error_expression.expr)?;

                if old_php_die != self.die || old_warnings.len() != self.warnings.len() {
                    self.die = old_php_die;
                    self.warnings = old_warnings.to_vec();
                }

                Ok(NULL)
            }
            Expression::Identifier(identifier) => match identifier {
                Identifier::SimpleIdentifier(simple_identifier) => {
                    let identifier_name = &simple_identifier.value;

                    let expr = self.env.get_ident(identifier_name);

                    if let Some(expr) = expr {
                        Ok(expr.clone())
                    } else {
                        let error = format!("Identifier {} not found", identifier_name,);

                        Err(PhpError {
                            level: ErrorLevel::Fatal,
                            message: error,
                            line: simple_identifier.span.line,
                        })
                    }
                }
                _ => todo!(),
            },
            Expression::Variable(var) => self.get_var(var),
            Expression::Include(include) => {
                self.handle_include(&include.path, false, include.include)
            }
            Expression::IncludeOnce(include) => {
                self.handle_include(&include.path, true, include.include_once)
            }
            Expression::Require(require) => {
                self.handle_require(&require.path, false, require.require)
            }
            Expression::RequireOnce(require) => {
                self.handle_require(&require.path, true, require.require_once)
            }
            Expression::FunctionCall(call) => function_call::expression(self, call),
            Expression::FunctionClosureCreation(_) => todo!(),
            Expression::Bool(b) => Ok(PhpValue::Bool(b.value)),
            _ => Ok(NULL),
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
        span: &Span,
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

    fn get_variable_name(&mut self, variable: &Variable) -> Result<Vec<u8>, PhpError> {
        match variable {
            Variable::SimpleVariable(sv) => Ok(sv.name.bytes.clone()),
            Variable::VariableVariable(vv) => {
                let value = self.get_variable_value(&vv.variable)?;

                if let PhpValue::String(value) = value {
                    Ok(value.bytes)
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
                let expr_value = self.eval_expression(&bvv.variable)?;

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

    fn get_variable_value(&mut self, variable: &Variable) -> Result<PhpValue, PhpError> {
        match variable {
            Variable::SimpleVariable(sv) => {
                let var_name = &sv.name.bytes;

                let value = self.env.get_var(var_name);

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

                    Ok(NULL)
                }
            }
            Variable::VariableVariable(vv) => self.get_var(&vv.variable),
            Variable::BracedVariableVariable(bvv) => {
                let expr_value = self.eval_expression(&bvv.variable)?;

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

                    return Ok(NULL);
                }

                let variable_name = expr_as_string.unwrap();

                if !self.env.var_exists(variable_name.as_bytes()) {
                    self.warnings.push(PhpError {
                        level: ErrorLevel::Warning,
                        message: format!("Undefined variable $ on line {}", bvv.start.line),
                        line: bvv.start.line,
                    });

                    return Ok(NULL);
                }

                Ok(self.env.get_var(variable_name.as_bytes()).unwrap())
            }
        }
    }

    fn change_var_value(
        &mut self,
        left_expr: &Expression,
        span: &Span,
        right_expr: &Expression,
        operation: &str,
    ) -> Result<PhpValue, PhpError> {
        let left = left_expr;
        let right = right_expr;

        let right_value = self.eval_expression(right)?;

        let Expression::Variable(ref var) = *left else {
			unreachable!()
        };

        let var_name = self.get_variable_name(var)?;

        let current_var_value = self.env.get_var(&var_name);

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
            _ => Ok(NULL),
        }?;

        if !self.env.var_exists(&var_name) {
            self.env.insert_var(&var_name, &new_value);
        } else {
            let old_value = self.env.get_var_with_rc(&var_name).unwrap();

            *old_value.borrow_mut() = new_value.clone();
        }

        Ok(new_value)
    }

    /// Returns the value of the variable. If it does not exist, the warning is added and Null is returned.
    fn get_var(&mut self, variable: &Variable) -> Result<PhpValue, PhpError> {
        let var_name = self.get_variable_name(variable)?;

        let value = self.env.get_var(&var_name);

        if let Some(value) = value {
            Ok(value)
        } else {
            let warning = format!("Undefined variable {}", get_string_from_bytes(&var_name));

            self.warnings.push(PhpError {
                level: ErrorLevel::Warning,
                message: warning,
                line: get_span_from_var(variable).line,
            });

            Ok(NULL)
        }
    }

    fn handle_include(
        &mut self,
        path: &Expression,
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

            return Ok(NULL);
        }

        let ok_abs_path = real_abs_path.unwrap();

        let path = ok_abs_path.to_str().unwrap();

        if once && self.included_files.iter().any(|i| *i == path) {
            return Ok(PhpValue::Bool(true));
        }

        let content = fs::read_to_string(path);

        if let Err(error) = content {
            let warning = PhpError {
                level: ErrorLevel::Warning,
                message: format!("{}({}): Failed to open stream: {}", fn_name, path, error),
                line: span.line,
            };

            self.warnings.push(warning);

            return Ok(NULL);
        }

        self.included_files.push(path.to_string());

        parse_php_file(self, path, &content.unwrap())
    }

    fn handle_require(
        &mut self,
        path: &Expression,
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

            return Ok(NULL);
        }

        let ok_abs_path = real_abs_path.unwrap();

        let path = ok_abs_path.to_str().unwrap();

        if once && self.required_files.iter().any(|i| *i == path) {
            return Ok(PhpValue::Bool(true));
        }

        let content = fs::read_to_string(path);

        if let Err(error) = content {
            let error = PhpError {
                level: ErrorLevel::Fatal,
                message: format!("{}({}): Failed to open stream: {}", fn_name, path, error),
                line: span.line,
            };

            self.warnings.push(error);

            return Ok(NULL);
        }

        self.required_files.push(path.to_string());

        parse_php_file(self, path, &content.unwrap())
    }
}
