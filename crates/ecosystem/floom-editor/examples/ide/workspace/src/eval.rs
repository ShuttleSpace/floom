//! The evaluator: a tree walk with an environment.

use std::collections::HashMap;

use crate::parser::{Expr, Op};

#[derive(Default)]
pub struct Evaluator {
    environment: HashMap<String, i64>,
}

impl Evaluator {
    pub fn eval(&mut self, program: &[Expr]) -> Result<i64, String> {
        let mut last = 0;

        for statement in program {
            last = self.expression(statement)?;
        }
        Ok(last)
    }

    fn expression(&mut self, expr: &Expr) -> Result<i64, String> {
        match expr {
            Expr::Number(value) => Ok(*value),
            Expr::Name(name) => self
                .environment
                .get(name)
                .copied()
                .ok_or_else(|| format!("`{name}` is not bound")),
            Expr::Negate(inner) => Ok(-self.expression(inner)?),
            Expr::Binary { op, left, right } => match op {
                // A binding is written as `name = value` and parsed as an
                // ordinary binary expression, so the evaluator is where the
                // left side stops being a value and becomes a place.
                Op::Equal => {
                    let value = self.expression(right)?;
                    if let Expr::Name(name) = &**left {
                        self.environment.insert(name.clone(), value);
                        Ok(value)
                    } else {
                        Err("the left side of `=` must be a name".to_string())
                    }
                }
                Op::Add => Ok(self.expression(left)? + self.expression(right)?),
                Op::Sub => Ok(self.expression(left)? - self.expression(right)?),
                Op::Mul => Ok(self.expression(left)? * self.expression(right)?),
                Op::Div => {
                    let divisor = self.expression(right)?;
                    if divisor == 0 {
                        return Err("division by zero".to_string());
                    }
                    Ok(self.expression(left)? / divisor)
                }
                Op::Less => Ok(i64::from(self.expression(left)? < self.expression(right)?)),
            },
        }
    }
}
