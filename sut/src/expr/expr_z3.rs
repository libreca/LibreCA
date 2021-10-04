// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::collections::HashMap;

use z3::ast::Bool;

use common::{Id, UVec};

use crate::expr::{BinOp, BOp, Eq, False, Not, True};

pub(crate) struct CHelpers<'ctx, 'sut> {
    pub(crate) context: &'ctx z3::Context,
    pub(crate) parameter_to_id: &'sut HashMap<String, usize>,
    pub(crate) value_to_id: &'sut UVec<HashMap<String, usize>>,
    pub(crate) value_testers: Vec<Vec<Bool<'ctx>>>,
}

pub(crate) trait ApplyZ3 {
    fn apply_z3<'ctx, 'sut>(&self, helpers: &CHelpers<'ctx, 'sut>) -> Bool<'ctx>;
}

impl ApplyZ3 for False {
    fn apply_z3<'ctx, 'sut>(&self, helpers: &CHelpers<'ctx, 'sut>) -> Bool<'ctx> {
        Bool::from_bool(helpers.context, false)
    }
}

impl ApplyZ3 for True {
    fn apply_z3<'ctx, 'sut>(&self, helpers: &CHelpers<'ctx, 'sut>) -> Bool<'ctx> {
        Bool::from_bool(helpers.context, true)
    }
}

impl ApplyZ3 for Not {
    fn apply_z3<'ctx, 'sut>(&self, helpers: &CHelpers<'ctx, 'sut>) -> Bool<'ctx> {
        self.sub.apply_z3(helpers).not()
    }
}

impl ApplyZ3 for BinOp {
    fn apply_z3<'ctx, 'sut>(&self, helpers: &CHelpers<'ctx, 'sut>) -> Bool<'ctx> {
        let left = self.left.apply_z3(helpers);
        let right = self.right.apply_z3(helpers);
        match self.op {
            BOp::And => Bool::and(helpers.context, &[&left, &right]),
            BOp::Or => Bool::or(helpers.context, &[&left, &right]),
            BOp::Implies => left.implies(&right),
        }
    }
}

impl ApplyZ3 for Eq {
    fn apply_z3<'ctx, 'sut>(&self, helpers: &CHelpers<'ctx, 'sut>) -> Bool<'ctx> {
        let parameter_id = helpers.parameter_to_id.get(&self.parameter).expect("Unknown parameter!");
        let value_id = helpers.value_to_id[(*parameter_id).as_usize()].get(&self.value).expect("Unknown value!");
        helpers.value_testers[(*parameter_id).as_usize()][(*value_id).as_usize()].clone()
    }
}
