// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fmt::{Debug, Error, Formatter};

#[cfg(feature = "constraints-z3")]
pub(crate) mod expr_z3;

#[cfg(not(feature = "constraints-z3"))]
mod expr_z3 {
    use super::*;

    pub(crate) trait ApplyZ3 {}

    impl ApplyZ3 for False {}

    impl ApplyZ3 for True {}

    impl ApplyZ3 for Not {}

    impl ApplyZ3 for BinOp {}

    impl ApplyZ3 for Eq {}
}

#[cfg(feature = "constraints-minisat")]
pub(crate) mod expr_minisat;

#[cfg(not(feature = "constraints-minisat"))]
mod expr_minisat {
    use super::*;

    pub(crate) trait ApplyMiniSat {}

    impl ApplyMiniSat for False {}

    impl ApplyMiniSat for True {}

    impl ApplyMiniSat for Not {}

    impl ApplyMiniSat for BinOp {}

    impl ApplyMiniSat for Eq {}
}


#[derive(Copy, Clone)]
pub(crate) enum BOp {
    And,
    Or,
    Implies,
}

impl std::fmt::Debug for BOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(match self {
            BOp::And => " && ",
            BOp::Or => " || ",
            BOp::Implies => " => ",
        })
    }
}

pub(crate) trait Expr: Debug + Send + Sync + expr_z3::ApplyZ3 + expr_minisat::ApplyMiniSat {
    fn fmt_no_parenthesis(&self, f: &mut Formatter<'_>) -> Result<(), Error>;
}

pub(crate) struct False;

impl Expr for False {
    fn fmt_no_parenthesis(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.fmt(f)
    }
}

impl Debug for False {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("False")
    }
}

pub(crate) struct True;

impl Expr for True {
    fn fmt_no_parenthesis(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.fmt(f)
    }
}

impl Debug for True {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("True")
    }
}

pub(crate) struct Not {
    pub(crate) sub: Box<dyn Expr>,
}

impl Expr for Not {
    fn fmt_no_parenthesis(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("!(")
            .and_then(|_| self.sub.fmt_no_parenthesis(f))
            .and_then(|_| f.write_str(")"))
    }
}

impl Debug for Not {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.fmt_no_parenthesis(f)
    }
}

pub(crate) struct BinOp {
    pub(crate) left: Box<dyn Expr>,
    pub(crate) op: BOp,
    pub(crate) right: Box<dyn Expr>,
}

impl Expr for BinOp {
    fn fmt_no_parenthesis(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.left.fmt(f)
            .and_then(|_| self.op.fmt(f))
            .and_then(|_| self.right.fmt(f))
    }
}

impl Debug for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("(")
            .and_then(|_| self.fmt_no_parenthesis(f))
            .and_then(|_| f.write_str(")"))
    }
}

pub(crate) struct Eq {
    pub(crate) parameter: String,
    pub(crate) value: String,
}

impl Expr for Eq {
    fn fmt_no_parenthesis(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.fmt(f)
    }
}

impl Debug for Eq {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str(&self.parameter)
            .and_then(|_| f.write_str("="))
            .and_then(|_| f.write_str(&self.value))
    }
}
