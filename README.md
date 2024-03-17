# phpl - A PHP Interpreter in Rust (Work in Progress)

phpl is an ongoing project aimed at developing a PHP interpreter in Rust. Please note that this project is in its early stages and cannot currently execute complete PHP files.

## Project Overview

-   **Project Status:** Work in Progress
-   **Current Capabilities:** Can parse and evaluate simple PHP expressions and statements.
-   **Language Compatibility:** Supports a subset of PHP features.

## Project Progress Checklist

These are all the statements/expressions currently supported:

-   [x] FullOpeningTag
-   [x] ShortOpeningTag
-   [x] EchoOpeningTag
-   [x] ClosingTag
-   [x] InlineHtml
-   [ ] Label
-   [ ] Goto
-   [ ] HaltCompiler
-   [ ] Static
-   [ ] DoWhile
-   [ ] While
-   [ ] For
-   [ ] Foreach
-   [ ] Break
-   [ ] Continue
-   [ ] Constant
-   [ ] Function
-   [ ] Class
-   [ ] Trait
-   [ ] Interface
-   [ ] If
-   [ ] Switch
-   [ ] Echo
-   [x] Expression
    -   [ ] Eval
    -   [x] Empty
    -   [x] Die
    -   [x] Exit
    -   [x] Isset
    -   [x] Unset
    -   [x] Print
    -   [x] Literal
    -   [x] ArithmeticOperation
    -   [x] AssignmentOperation
    -   [x] BitwiseOperation
    -   [x] ComparisonOperation
    -   [x] LogicalOperation
    -   [x] Concat
    -   [x] Instanceof
    -   [x] Reference
    -   [x] Parenthesized
    -   [x] ErrorSuppress
    -   [x] Identifier
    -   [x] Variable
    -   [x] Include
    -   [x] IncludeOnce
    -   [x] Require
    -   [x] RequireOnce
    -   [ ] FunctionCall
    -   [ ] FunctionClosureCreation
    -   [ ] MethodCall
    -   [ ] MethodClosureCreation
    -   [ ] NullsafeMethodCall
    -   [ ] StaticMethodCall
    -   [ ] StaticVariableMethodCall
    -   [ ] StaticMethodClosureCreation
    -   [ ] StaticVariableMethodClosureCreation
    -   [ ] PropertyFetch
    -   [ ] NullsafePropertyFetch
    -   [ ] StaticPropertyFetch
    -   [ ] ConstantFetch
    -   [ ] Static
    -   [ ] Self\_
    -   [ ] Parent
    -   [ ] ShortArray
    -   [ ] Array
    -   [ ] List
    -   [ ] Closure
    -   [ ] ArrowFunction
    -   [ ] New
    -   [ ] InterpolatedString
    -   [ ] Heredoc
    -   [ ] Nowdoc
    -   [ ] ShellExec
    -   [ ] AnonymousClass
    -   [x] Bool
    -   [ ] ArrayIndex
    -   [ ] Null
    -   [ ] MagicConstant
    -   [ ] ShortTernary
    -   [ ] Ternary
    -   [ ] Coalesce
    -   [ ] Clone
    -   [ ] Match
    -   [ ] Throw
    -   [ ] Yield
    -   [ ] YieldFrom
    -   [ ] Cast
    -   [ ] Noop
-   [ ] Return
-   [ ] Namespace
-   [ ] Use
-   [ ] GroupUse
-   [ ] Comment
-   [ ] Try
-   [ ] UnitEnum
-   [ ] BackedEnum
-   [ ] Block
-   [ ] Global
-   [ ] Declare
-   [ ] Noop

## Cloning and Running the Project

To use this project, follow these steps:

1. **Clone the Repository:** Begin by cloning the repository from the Git repository using the following command:

    ```bash
    git clone https://github.com/Davidflogar/phpl
    cd phpl
    cargo r file.php # or you can build the project
    ```

## Differences between phpl and the normal php interpreter

1. Declaring variables does not return any value. Example in normal php:

    ```php
    $b = $a = 1;
    // $b is 1
    ```

    Example in phpl:

    ```php
    $b = $a = 1;
    // $b is null
    ```

2. When instantiating a class in phpl, after executing the constructor, the constructor is deleted, although the function still exists, the body will be empty.

3. PHPL will not attempt to convert a parameter to the correct data type when passed to a function. For example: if a function receives an integer, the data type of the passed parameter must be an integer and no attempt will be made to convert the parameter to an integer. That's how it is with all data types.

4. When using promoted properties, the variable will be a reference to the property of that same object:
    ```php
    class A {
    	public function __construct(public mixed $a) {
			// here $a is the same as $this->a, so any change to one of the two variables will be reflected in the other
		}
    }
    ```

5. A boolean value that is `false` in string will be represented as `0` and not as an empty string. Similarly, a `null` value as a string will be the text `null`.

6. A string that contains only a `0` will not be considered empty by the `empty` function.

7. When creating a reference to a value, the referenced value will also become a reference. Example:
	```php
	$a = 1;
	$b = &$a; // here assign $b a reference to $a internally mark $a as reference
	```

There are still some undocumented differences, so this list will expand over time.
