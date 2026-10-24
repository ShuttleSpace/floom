# Design notes

The language is deliberately small: expressions, `let` bindings, and
arithmetic. It exists so the demo has plausible source to read.

## Why three stages

A lexer, a parser and an evaluator keep each stage small enough to read in one
sitting, and each one fails in a different way:

1. **Lexing** fails on characters that cannot start a token.
2. **Parsing** fails on a well-formed token in the wrong place.
3. **Evaluation** fails on a name that was never bound.

Separating the failures is what makes the error messages useful.

## Precedence

| Level | Operators | Associativity |
| --- | --- | --- |
| 1 | `*` `/` | left |
| 2 | `+` `-` | left |
| 3 | `==` `<` | left |

Parentheses override all of it. Unary minus binds tighter than `*`.

> A parser that does not encode precedence is a parser that has one.

```rust
let expr = Expr::binary(Op::Add, Expr::number(1), Expr::number(2));
assert_eq!(evaluate(&expr), Ok(3));
```
