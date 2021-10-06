// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the [ValueGenerator], which is used to iterate over the values of a PC.

use crate::{Number, UVec};

/// This struct is for iterating over the interactions of a given PC.
pub struct ValueGenerator<ValueId: Number, const STRENGTH: usize> {
    /// This array contains the maximum value of each parameter in the PC.
    pub max_values: [ValueId; STRENGTH],
}

impl<ValueId: Number, const STRENGTH: usize>
ValueGenerator<ValueId, STRENGTH>
    where [(); STRENGTH - 1]:
{
    /// Initialise the struct for the provided PC.
    pub fn new<ParameterId: Number>(
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc: &[ParameterId; STRENGTH - 1],
    ) -> Self {
        let mut max_values = [ValueId::default(); STRENGTH];
        for index in 0..STRENGTH - 1 {
            max_values[index] = parameters[pc[index].as_usize()];
        }
        max_values[STRENGTH - 1] = parameters[at_parameter];

        Self { max_values }
    }

    /// Set the next values in the given array.
    pub fn next_array(&self, values: &mut [ValueId; STRENGTH]) -> bool {
        let mut index = STRENGTH - 1;
        values[index] += ValueId::from_usize(1);

        while 0 < index && values[index] == self.max_values[index] {
            values[index] = ValueId::default();
            values[index - 1] += ValueId::from_usize(1);
            index -= 1;
        }

        values[0] != self.max_values[0]
    }

    /// Skip the given number of values.
    pub fn skip_array(&self, values: &mut [ValueId; STRENGTH], skip: ValueId) -> bool {
        let mut index = STRENGTH - 1;
        values[index] += skip;

        let mut value = values[index];
        let mut max_value = self.max_values[index];

        while 0 < index && value >= max_value {
            values[index] = value % max_value;
            values[index - 1] += value / max_value;
            index -= 1;

            value = values[index];
            max_value = self.max_values[index]
        }

        index != 0 || value < max_value
    }

    /// Set the next value in the given vector.
    pub fn next_vector(&self, values: &mut UVec<ValueId>) -> bool {
        let mut index = STRENGTH - 1;
        values[index] += ValueId::from_usize(1);

        while 0 < index && values[index] == self.max_values[index] {
            values[index] = ValueId::default();
            values[index - 1] += ValueId::from_usize(1);
            index -= 1;
        }

        values[0] != self.max_values[0]
    }

    /// Set the next value in the given vector, but increase the first elements first.
    pub fn next_vector_inverse(&self, values: &mut UVec<ValueId>) -> bool {
        let mut index = 0;
        values[index] += ValueId::from_usize(1);

        while index < STRENGTH - 1 && values[index] == self.max_values[index] {
            values[index] = ValueId::default();
            values[index + 1] += ValueId::from_usize(1);
            index += 1;
        }

        values[STRENGTH - 1] != self.max_values[STRENGTH - 1]
    }
}
