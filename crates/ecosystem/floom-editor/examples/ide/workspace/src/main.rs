//! A tiny expression language, interpreted.
//!
//! This program exists to be read in the demo, not to be run. It lexes,
//! parses and evaluates arithmetic with variables.

mod eval;
mod lexer;
mod parser;

use std::env;

use eval::Evaluator;
use lexer::Lexer;
use parser::Parser;

fn main() {
    let source = env::args()
        .nth(1)
        .unwrap_or_else(|| "let answer = 6 * 7; answer + 1".to_string());

    match run(&source) {
        Ok(value) => println!("{source} = {value}"),
        Err(error) => eprintln!("error: {error}"),
    }
}

/// Lexes, parses and evaluates one program.
fn run(source: &str) -> Result<i64, String> {
    let tokens = Lexer::new(source).tokens()?;
    let program = Parser::new(tokens).program()?;
    let mut evaluator = Evaluator::default();

    evaluator.eval(&program)
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn arithmetic_binds_tighter_than_equality() {
        assert_eq!(run("1 + 2 * 3").unwrap(), 7);
        assert_eq!(run("(1 + 2) * 3").unwrap(), 9);
    }

    #[test]
    fn variables_are_remembered() {
        assert_eq!(run("let x = 4; x * x").unwrap(), 16);
    }
}
