// Compile and execute .ect source files
use ect::lexer::Lexer;
use ect::parser::Parser;
use ect::compiler::Compiler;
use ect::vm::{Value, VirtualMachine};
use std::fs;

fn compile_and_run_file(path: &str) -> Result<Value, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut parser = Parser::new(Lexer::new(&source)).with_source(source.clone());
    let ast = parser.parse_program();

    if !parser.errors.is_empty() {
        return Err(format!("Parser errors:\n{}", parser.pretty_print_errors()));
    }

    if ast.is_empty() {
        return Err("Parser produced empty AST".to_string());
    }

    let mut compiler = Compiler::new().with_source(source);
    let module = compiler.compile_module(&ast)
        .map_err(|_| compiler.pretty_print_errors())?;

    let mut vm = VirtualMachine::new();
    vm.run_module(&module)
        .map_err(|e| format!("VM error: {}", e))
}

#[test]
fn test_fibonacci_10() {
    let result = compile_and_run_file("tests/scripts/fibonacci.ect");
    assert_eq!(result, Ok(Value::Int(55)), "fib(10) should be 55");
}

#[test]
fn test_fibonacci_5() {
    let source = r#"
fn fib(n: Int) -> Int {
  if n <= 1 {
    n
  } else {
    fib(n - 1) + fib(n - 2)
  }
}

fn main() -> Int {
  fib(5)
}
"#;

    let mut parser = Parser::new(Lexer::new(source));
    let ast = parser.parse_program();
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("Compiler failed");
    let mut vm = VirtualMachine::new();
    let result = vm.run_module(&module).expect("VM failed");

    assert_eq!(result, Value::Int(5), "fib(5) should be 5");
}

#[test]
fn test_factorial_5() {
    let result = compile_and_run_file("tests/scripts/factorial.ect");
    assert_eq!(result, Ok(Value::Int(120)), "factorial(5) should be 120");
}

#[test]
fn test_factorial_0() {
    let source = r#"
fn factorial(n: Int) -> Int {
  if n <= 1 {
    1
  } else {
    n * factorial(n - 1)
  }
}

fn main() -> Int {
  factorial(0)
}
"#;

    let mut parser = Parser::new(Lexer::new(source));
    let ast = parser.parse_program();
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("Compiler failed");
    let mut vm = VirtualMachine::new();
    let result = vm.run_module(&module).expect("VM failed");

    assert_eq!(result, Value::Int(1), "factorial(0) should be 1");
}

#[test]
fn test_sum_loop() {
    let result = compile_and_run_file("tests/scripts/sum_loop.ect");
    assert_eq!(result, Ok(Value::Int(55)), "sum_to(10) should be 55 (1+2+...+10)");
}

#[test]
fn test_power() {
    let result = compile_and_run_file("tests/scripts/power.ect");
    assert_eq!(result, Ok(Value::Int(1024)), "power(2, 10) should be 1024");
}

#[test]
fn test_power_of_3() {
    let source = r#"
fn power(base: Int, exp: Int) -> Int {
  if exp <= 0 {
    1
  } else {
    base * power(base, exp - 1)
  }
}

fn main() -> Int {
  power(3, 5)
}
"#;

    let mut parser = Parser::new(Lexer::new(source));
    let ast = parser.parse_program();
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("Compiler failed");
    let mut vm = VirtualMachine::new();
    let result = vm.run_module(&module).expect("VM failed");

    assert_eq!(result, Value::Int(243), "power(3, 5) should be 243");
}

#[test]
fn test_nested_conditionals() {
    let source = r#"
fn classify(n: Int) -> Int {
  if n < 0 {
    0
  } else {
    if n == 0 {
      1
    } else {
      if n < 10 {
        2
      } else {
        3
      }
    }
  }
}

fn main() -> Int {
  classify(5)
}
"#;

    let mut parser = Parser::new(Lexer::new(source));
    let ast = parser.parse_program();
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("Compiler failed");
    let mut vm = VirtualMachine::new();
    let result = vm.run_module(&module).expect("VM failed");

    assert_eq!(result, Value::Int(2), "classify(5) should be 2");
}

#[test]
fn test_mutual_recursion() {
    let source = r#"
fn is_even(n: Int) -> Int {
  if n == 0 {
    1
  } else {
    is_odd(n - 1)
  }
}

fn is_odd(n: Int) -> Int {
  if n == 0 {
    0
  } else {
    is_even(n - 1)
  }
}

fn main() -> Int {
  is_even(6)
}
"#;

    let mut parser = Parser::new(Lexer::new(source));
    let ast = parser.parse_program();
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("Compiler failed");
    let mut vm = VirtualMachine::new();
    let result = vm.run_module(&module).expect("VM failed");

    assert_eq!(result, Value::Int(1), "is_even(6) should be 1 (true)");
}

#[test]
fn test_match_dispatch() {
    let source = r#"
fn quadrant(x: Int, y: Int) -> Int {
  match x {
    0 => 0
    1 => match y {
      1 => 1
      2 => 2
      _ => 0
    }
    2 => match y {
      1 => 3
      2 => 4
      _ => 0
    }
    _ => 0
  }
}

fn main() -> Int {
  quadrant(2, 1)
}
"#;

    let mut parser = Parser::new(Lexer::new(source));
    let ast = parser.parse_program();
    let mut compiler = Compiler::new();
    let module = compiler.compile_module(&ast).expect("Compiler failed");
    let mut vm = VirtualMachine::new();
    let result = vm.run_module(&module).expect("VM failed");

    assert_eq!(result, Value::Int(3), "quadrant(2, 1) should be 3");
}
