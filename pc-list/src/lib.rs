// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the [PCList] struct.

#![cfg_attr(test, feature(test))]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

use common::{Id, UVec};

#[cfg(test)]
mod test_gen;

type DontCareArray = u64;

/// This struct contains all the PCs (Parameter Combinations) for the entire generation.
#[derive(Clone)]
pub struct PCList<ParameterId: Id, const STRENGTH: usize>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// These are the actual PCs provided by this struct.
    ///
    /// Should not change after [PCList::new] creates it.
    pub pcs: UVec<[ParameterId; STRENGTH - 1]>,

    /// This vector contains the parameter locations for each PC.
    pub locations: UVec<DontCareArray>,

    /// This vector contains the number of PCs for each iteration of the IPOG algorithm.
    ///
    /// To get the current number of PCs:
    /// ```
    /// # use pc_list::PCList;
    /// # let pc_list = PCList::<u8, 6>::new(10);
    /// # let STRENGTH: usize = 6;
    /// # let at_parameter: usize = 7;
    /// let pc_list_len = pc_list.sizes[at_parameter - STRENGTH];
    /// ```
    pub sizes: UVec<usize>,
}

impl<ParameterId: Id, const STRENGTH: usize>
PCList<ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    /// Create a new [PCList] struct for the given parameter_count.
    pub fn new(parameter_count: usize) -> Self {
        let pc_list_len = calculate_length(STRENGTH, parameter_count - 1);
        let pcs_sizes_len = parameter_count - STRENGTH;
        let mut pc: UVec<[ParameterId; STRENGTH - 1]> = UVec::with_capacity(pc_list_len);
        let mut sizes: UVec<usize> = UVec::with_capacity(pcs_sizes_len);
        let mut locations: UVec<DontCareArray> = UVec::with_capacity(pc_list_len);

        if STRENGTH == 2 {
            pc.push([ParameterId::default(); STRENGTH - 1]);
            let mut location = 1;
            locations.push(location);

            for at_parameter in STRENGTH..parameter_count {
                pc.push([ParameterId::from_usize(at_parameter - 1); STRENGTH - 1]);
                sizes.push(pc.len());
                location <<= 1;
                locations.push(location);
            }
        } else {
            let mut current_pc = [ParameterId::default(); STRENGTH - 1];

            for (at_parameter, value) in current_pc.iter_mut().enumerate().take(STRENGTH - 1) {
                *value = ParameterId::from_usize(at_parameter);
            }

            pc.push(current_pc.clone());
            locations.push(Self::pc_to_locations(&current_pc));

            for at_parameter in STRENGTH..parameter_count {
                // initialise the first combination
                current_pc[0] = ParameterId::default();
                current_pc[STRENGTH - 2] = ParameterId::from_usize(at_parameter - 1);

                let mut index: usize = 0;

                // current_pc[0] can only be "parameters_len - STRENGTH + 1" exactly once - our termination condition!
                while index != 0 || (at_parameter + 2 - STRENGTH) > current_pc[0].as_usize() {
                    // Reset each outer element to prev element + 1
                    while index < STRENGTH - 3 {
                        current_pc[index + 1] =
                            current_pc[index] + ParameterId::from_usize(1);
                        index += 1;
                    }

                    pc.push(current_pc.clone());
                    locations.push(Self::pc_to_locations(&current_pc));

                    // If outer elements are saturated, keep decrementing index till you find unsaturated element
                    while index > 0
                        && current_pc[index].as_usize() == (at_parameter + 1 - STRENGTH + index)
                    {
                        index -= 1;
                    }

                    current_pc[index] += ParameterId::from_usize(1);
                }

                sizes.push(pc.len());
            }
        }

        Self { pcs: pc, locations, sizes }
    }

    fn pc_to_locations(pc: &[ParameterId; STRENGTH - 1]) -> DontCareArray {
        let mut location: DontCareArray = 0;
        for &parameter_id in pc.iter() {
            location += 1 << parameter_id.as_usize() as DontCareArray;
        }
        location
    }
}

/// Calculate the number of PCs for the given strength and number of parameters.
#[inline]
pub fn calculate_length(mut strength: usize, at_parameter: usize) -> usize {
    strength -= 1;

    let mut res: usize = 1;
    if strength > at_parameter - strength {
        strength = at_parameter - strength;
    }
    for i in 0..strength {
        res *= at_parameter - i;
        res /= i + 1;
    }
    res
}
