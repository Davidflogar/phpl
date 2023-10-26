# phpl - A PHP Interpreter in Rust (Work in Progress)

phpl is an ongoing project aimed at developing a PHP interpreter in Rust. Please note that this project is in its early stages and cannot currently execute complete PHP files. The primary goal of this project is to deepen my understanding of Rust programming while exploring the intricacies of building a PHP interpreter.

## Project Overview

- **Project Status:** Work in Progress
- **Current Capabilities:** Can parse and evaluate simple PHP expressions and statements.
- **Language Compatibility:** Supports a subset of PHP features.

## Project Progress Checklist

These are all the statements/expressions currently supported:

- [x] FullOpeningTag
- [x] ShortOpeningTag
- [x] EchoOpeningTag
- [ ] ClosingTag
- [x] InlineHtml
- [ ] Label
- [ ] Goto
- [ ] HaltCompiler
- [ ] Static
- [ ] DoWhile
- [ ] While
- [ ] For
- [ ] Foreach
- [ ] Break
- [ ] Continue
- [ ] Constant
- [ ] Function
- [ ] Class
- [ ] Trait
- [ ] Interface
- [ ] If
- [ ] Switch
- [ ] Echo
- [x] Expression
	- [ ] Eval
	- [x] Empty
	- [x] Die
	- [x] Exit
	- [x] Isset
	- [x] Unset
	- [x] Print
	- [x] Literal
	- [x] ArithmeticOperation
	- [x] AssignmentOperation
	- [x] BitwiseOperation
	- [x] ComparisonOperation
	- [x] LogicalOperation
	- [x] Concat
	- [x] Instanceof
	- [x] Reference
	- [x] Parenthesized
	- [ ] ErrorSuppress
	- [ ] Identifier
	- [x] Variable
	- [ ] Include
	- [ ] IncludeOnce
	- [ ] Require
	- [ ] RequireOnce
	- [ ] FunctionCall
	- [ ] FunctionClosureCreation
	- [ ] MethodCall
	- [ ] MethodClosureCreation
	- [ ] NullsafeMethodCall
	- [ ] StaticMethodCall
	- [ ] StaticVariableMethodCall
	- [ ] StaticMethodClosureCreation
	- [ ] StaticVariableMethodClosureCreation
	- [ ] PropertyFetch
	- [ ] NullsafePropertyFetch
	- [ ] StaticPropertyFetch
	- [ ] ConstantFetch
	- [ ] Static
	- [ ] Self_
	- [ ] Parent
	- [ ] ShortArray
	- [ ] Array
	- [ ] List
	- [ ] Closure
	- [ ] ArrowFunction
	- [ ] New
	- [ ] InterpolatedString
	- [ ] Heredoc
	- [ ] Nowdoc
	- [ ] ShellExec
	- [ ] AnonymousClass
	- [x] Bool
	- [ ] ArrayIndex
	- [ ] Null
	- [ ] MagicConstant
	- [ ] ShortTernary
	- [ ] Ternary
	- [ ] Coalesce
	- [ ] Clone
	- [ ] Match
	- [ ] Throw
	- [ ] Yield
	- [ ] YieldFrom
	- [ ] Cast
	- [ ] Noop
- [ ] Return
- [ ] Namespace
- [ ] Use
- [ ] GroupUse
- [ ] Comment
- [ ] Try
- [ ] UnitEnum
- [ ] BackedEnum
- [ ] Block
- [ ] Global
- [ ] Declare
- [ ] Noop

## Cloning and Running the Project

To use this project, follow these steps:

1. **Clone the Repository:** Begin by cloning the repository from the Git repository using the following command:

   ```bash
   git clone https://github.com/Davidflogar/phpl
   cd phpl
   cargo r file.php // or you can build the project
