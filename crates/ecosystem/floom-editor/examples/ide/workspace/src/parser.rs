//! A recursive-descent parser.
//!
//! Precedence is encoded in the shape of the recursion: `expression` calls
//! `term`, and `term` calls `factor`. There is no precedence table to keep in
//! sync with the evaluator.

use crate::lexer::{Keyword, Kind, Token};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(i64),
    Name(String),
    Binary {
        op: Op,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Negate(Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    Less,
}

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    /// `let name = expr;` statements, then a final expression.
    pub fn program(&mut self) -> Result<Vec<Expr>, String> {
        let mut statements = Vec::new();

        while self.check_keyword(Keyword::Let) {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(Kind::Equal)?;
            let value = self.expression()?;
            self.expect(Kind::Semi)?;
            statements.push(Expr::Binary {
                op: Op::Equal,
                left: Box::new(Expr::Name(name)),
                right: Box::new(value),
            });
        }

        statements.push(self.expression()?);
        Ok(statements)
    }

    fn expression(&mut self) -> Result<Expr, String> {
        let mut left = self.term()?;

        while let Some(op) = self.match_op(&[(Kind::Plus, Op::Add), (Kind::Minus, Op::Sub)]) {
            let right = self.term()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.factor()?;

        while let Some(op) = self.match_op(&[(Kind::Star, Op::Mul), (Kind::Slash, Op::Div)]) {
            let right = self.factor()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, String> {
        if self.check(Kind::Minus) {
            self.advance();
            return Ok(Expr::Negate(Box::new(self.factor()?)));
        }
        match self.peek().clone() {
            Kind::Number(value) => {
                self.advance();
                Ok(Expr::Number(value))
            }
            Kind::Ident(name) => {
                self.advance();
                Ok(Expr::Name(name))
            }
            Kind::Open => {
                self.advance();
                let inner = self.expression()?;
                self.expect(Kind::Close)?;
                Ok(inner)
            }
            other => Err(format!("expected an expression, found {other}")),
        }
    }

    fn match_op(&mut self, ops: &[(Kind, Op)]) -> Option<Op> {
        for (kind, op) in ops {
            if self.check(kind.clone()) {
                self.advance();
                return Some(*op);
            }
        }
        None
    }

    fn expect(&mut self, kind: Kind) -> Result<(), String> {
        if self.check(kind.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {kind}, found {}", self.peek()))
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            Kind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(format!("expected a name, found {other}")),
        }
    }

    fn check(&self, kind: Kind) -> bool {
        self.peek() == &kind
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        self.peek() == &Kind::Keyword(keyword)
    }

    fn peek(&self) -> &Kind {
        &self.tokens[self.index.min(self.tokens.len() - 1)].kind
    }

    fn advance(&mut self) {
        self.index += 1;
    }
}
