// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the IPOG algorithm for [ConstrainedSUT]s.

#![allow(clippy::trivially_copy_pass_by_ref)]

use std::marker::PhantomData;

use cm::{BIT_MASK, BIT_SHIFT, BitArray, CoverageMap, get_highscore_blacklisted};
use common::{Id, sub_time_it, u_vec, UVec, ValueGenerator};
use mca::{check_locations, DontCareArray, MCA};
use pc_list::PCList;
use sut::{ConstrainedSUT, Solver};

/// This trait allows for the switching of various IPOG extension methods.
pub trait Extension<ValueId: Id, ParameterId: Id, const STRENGTH: usize> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Used for debugging purposes.
    const NAME: &'static str;

    /// Does the extension for the specified strength.
    unsafe fn extend<'a, S: Solver<'a>>(
        solver: &mut S,
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    );
}

/// Do not do anything for the extension.
pub struct NOOPExtension<const STRENGTH: usize>;

impl<ValueId: Id, ParameterId: Id, const STRENGTH: usize> Extension<ValueId, ParameterId, STRENGTH> for NOOPExtension<STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    const NAME: &'static str = "NOOP";

    unsafe fn extend<'a, S: Solver<'a>>(
        _solver: &mut S,
        _parameters: &UVec<ValueId>,
        _at_parameter: usize,
        _pc_list: &PCList<ParameterId, STRENGTH>,
        _pc_list_len: usize,
        _mca: &mut MCA<ValueId>,
        _coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {}
}

/// Run and time the extension if the `sub-time` feature is set. Runs the extension if the feature is not set.
pub struct TimedExtension<
    ValueId: Id,
    ParameterId: Id,
    SubExtension: Extension<ValueId, ParameterId, STRENGTH>,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    phantom: PhantomData<SubExtension>,
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
}

impl<
    ValueId: Id,
    ParameterId: Id,
    SubExtension: Extension<ValueId, ParameterId, STRENGTH>,
    const STRENGTH: usize,
> Extension<ValueId, ParameterId, STRENGTH>
for TimedExtension<ValueId, ParameterId, SubExtension, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = SubExtension::NAME;

    unsafe fn extend<'a, S: Solver<'a>>(
        solver: &mut S,
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {
        sub_time_it!(
            SubExtension::extend(
                solver,
                parameters,
                at_parameter,
                pc_list,
                pc_list_len,
                mca,
                coverage_map
            ),
            Self::NAME
        );
    }
}

/// The struct implementing the HorizontalExtension for the constrained version of IPOG.
pub struct HorizontalExtension<
    ValueId: Id,
    ParameterId: Id,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
}

/// The HorizontalExtension for the constrained version of IPOG.
impl<ValueId: Id, ParameterId: Id, const STRENGTH: usize>
HorizontalExtension<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    unsafe fn get_best_value<'a, S: Solver<'a>>(
        solver: &mut S,
        at_parameter: usize,
        mut previous_value: ValueId,
        value_choices: ValueId,
        scores: &mut UVec<UVec<BitArray>>,
        blacklist: &mut UVec<bool>,
        uses: &mut UVec<usize>,
    ) -> Option<ValueId> {
        for _ in 1..value_choices.as_usize() {
            // Try to fit the
            let value = get_highscore_blacklisted(&scores, &uses, previous_value, &blacklist);

            if scores[value.as_usize()].is_empty() {
                return None;
            }

            solver.push_and_assert_eq(ParameterId::from_usize(at_parameter), value);
            let valid = solver.check();
            solver.pop(1);

            if valid {
                return Some(value);
            } else {
                blacklist[value.as_usize()] = true;
                if value == previous_value {
                    while blacklist[previous_value.as_usize()] {
                        previous_value = (previous_value + ValueId::from_usize(1))
                            % (ValueId::from_usize(scores.len()));
                    }
                }
            }
        }

        let mut value = ValueId::default();
        while blacklist[value.as_usize()] {
            value += ValueId::from_usize(1);
        }

        Some(value)
    }
}

impl<ValueId: Id, ParameterId: Id, const STRENGTH: usize>
Extension<ValueId, ParameterId, STRENGTH>
for HorizontalExtension<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = "B HE";

    unsafe fn extend<'a, S: Solver<'a>>(
        solver: &mut S,
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {
        let dont_care_mask = !(1 << at_parameter as DontCareArray);

        coverage_map.set_zero_covered();
        let mut previous_value = ValueId::default();

        let value_choices = parameters[at_parameter];
        let mut scores = u_vec![UVec::with_capacity(pc_list_len); value_choices.as_usize()];
        let mut blacklist = u_vec![false; value_choices.as_usize()];
        let mut uses = u_vec![0; value_choices.as_usize()];
        uses[0] = 1;

        'row: for row_id in 1..mca.array.len() {
            let row = &mut mca.array[row_id].as_slice_mut();
            let dont_care_locations = &mut mca.dont_care_locations[row_id];

            for score in scores.iter_mut() {
                score.clear();
            }

            for b in blacklist.iter_mut() {
                *b = false;
            }

            coverage_map.get_high_score(&pc_list, pc_list_len, row, &mut scores);

            if scores.iter().all(UVec::is_empty) {
                continue 'row;
            }

            solver.push_and_assert_row(&row[..at_parameter]);

            let fill_row = Self::get_best_value(
                solver,
                at_parameter,
                previous_value,
                value_choices,
                &mut scores,
                &mut blacklist,
                &mut uses,
            );

            solver.pop(1); // Pop row

            if let Some(value) = fill_row {
                *row.get_unchecked_mut(at_parameter) = value;
                uses[value.as_usize()] += 1;
                *dont_care_locations &= dont_care_mask;
                previous_value = value;
                debug_assert!(check_locations(row, *dont_care_locations));

                coverage_map.set_indices(&scores[value.as_usize()]);

                if coverage_map.is_covered() {
                    return;
                }
            }
        }
    }
}

/// The VerticalExtension for the constrained version of IPOG.
pub struct VerticalExtension<
    ValueId: Id,
    ParameterId: Id,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
}

impl<ValueId: Id, ParameterId: Id, const STRENGTH: usize>
VerticalExtension<ValueId, ParameterId, STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    fn pc_valid<'a, S: Solver<'a>>(
        solver: &mut S,
        at_parameter: usize,
        pc: &[ParameterId; STRENGTH - 1],
        values: &[ValueId; STRENGTH],
    ) -> bool {
        solver.push_and_assert_interaction(pc, at_parameter, values);
        let result = solver.check();
        if !result {
            solver.pop_all(1);
        }
        result
    }

    #[inline]
    unsafe fn pc_fits_row<'a, S: Solver<'a>>(
        solver: &mut S,
        at_parameter: usize,
        pc: &[ParameterId; STRENGTH - 1],
        values: &[ValueId; STRENGTH],
        row: &mut [ValueId],
        dont_care_locations: DontCareArray,
        pc_locations: DontCareArray,
    ) -> bool {
        let shared_locations = dont_care_locations & pc_locations;
        if shared_locations == 0 {
            return false;
        }

        if shared_locations != pc_locations {
            for (&parameter_id, &value) in pc.iter().zip(values.iter()) {
                if *row.get_unchecked(parameter_id.as_usize()) != value
                    && *row.get_unchecked(parameter_id.as_usize()) != ValueId::dont_care()
                {
                    // If the interaction does not fit go to the next row
                    return false;
                }
            }
        }

        let last_value_interaction = *values.get_unchecked(STRENGTH - 1);
        let last_value_row = row.get_unchecked_mut(at_parameter);
        if *last_value_row != last_value_interaction && *last_value_row != ValueId::dont_care() {
            return false;
        }

        solver.push_and_assert_row_masked(row, pc, at_parameter);
        let last_value_row = row.get_unchecked_mut(at_parameter);
        let valid = solver.check_and_pop(1);

        if !valid {
            false
        } else if *last_value_row == last_value_interaction {
            true
        } else {
            *last_value_row = last_value_interaction;
            true
        }
    }

    unsafe fn fit_in_row<'a, S: Solver<'a>>(
        solver: &mut S,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
        pc: &[ParameterId; STRENGTH - 1],
        values: &[ValueId; STRENGTH],
        pc_id: usize,
        pc_locations_tuple: &(DontCareArray, DontCareArray),
        locations_mask: DontCareArray,
    ) -> bool {
        // iterate over all rows of the MCA
        for (ve_index, &row_id) in mca.vertical_extension_rows.iter().enumerate() {
            let row = mca.array[row_id].as_slice_mut();
            let dont_care_locations = &mut mca.dont_care_locations[row_id];

            if Self::pc_fits_row::<S>(
                solver,
                at_parameter,
                pc,
                values,
                row,
                *dont_care_locations,
                pc_locations_tuple.0,
            ) {
                // Interaction fits in the row, so fill the values in the row
                for (&parameter_id, &value) in pc.iter().zip(values.iter()) {
                    *row.get_unchecked_mut(parameter_id.as_usize()) = value;
                }

                *dont_care_locations &= pc_locations_tuple.1;

                if *dont_care_locations & locations_mask == 0 {
                    mca.vertical_extension_rows.remove(ve_index);
                }

                debug_assert!(check_locations(row, *dont_care_locations));

                coverage_map.set_covered_row_simple_sub(
                    at_parameter,
                    &pc_list,
                    row,
                    pc_id + 1,
                    pc_list_len,
                );

                // Done with this interaction, so stop iterating
                return true;
            }
        }

        false
    }
}

impl<ValueId: Id, ParameterId: Id, const STRENGTH: usize>
Extension<ValueId, ParameterId, STRENGTH>
for VerticalExtension<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = "B VE";

    unsafe fn extend<'a, S: Solver<'a>>(
        solver: &mut S,
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {
        debug_assert!(
            ((BIT_MASK + 1) as usize) < (ValueId::dont_care().as_usize()),
            "Bitarray and ValueId are incompatible."
        );
        let value_choices = parameters[at_parameter].as_usize() as BitArray;

        let locations_mask = mca.set_vertical_extension_rows(at_parameter);

        debug_assert!(mca.check_all(at_parameter));

        let parameter_mask = 1 << at_parameter as BitArray;

        for pc_id in 0..pc_list.pcs.len() {
            let pc = &pc_list.pcs[pc_id];

            let mut values = [ValueId::default(); STRENGTH];
            let value_generator = ValueGenerator::<ValueId, STRENGTH>::new(
                &parameters, at_parameter, pc,
            );
            let mut map_index = (coverage_map.sizes[pc_id][0] * value_choices) + 1;
            let mut pc_locations_option: Option<(DontCareArray, DontCareArray)> = None;

            'sup_index: loop {
                let map_sub_index = map_index & BIT_MASK;
                let mut map_array = coverage_map.map[map_index as usize >> BIT_SHIFT] >> map_sub_index;

                // Skip block if the block is covered
                if map_array == !0 {
                    if value_generator
                        .skip_array(&mut values, ValueId::from_usize(BIT_MASK as usize + 1))
                    {
                        map_index += BIT_MASK + 1;
                        continue 'sup_index;
                    } else {
                        break 'sup_index;
                    }
                }

                for _ in map_sub_index..=BIT_MASK {
                    if value_generator.next_array(&mut values) {
                        if map_array & 1 == 0 {
                            coverage_map.uncovered -= 1;

                            if Self::pc_valid(solver, at_parameter, pc, &values) {
                                let pc_locations_tuple = pc_locations_option.get_or_insert_with(|| {
                                    (pc_list.locations[pc_id], !(pc_list.locations[pc_id] | parameter_mask))
                                });

                                if !Self::fit_in_row(
                                    solver,
                                    at_parameter,
                                    pc_list,
                                    pc_list_len,
                                    mca,
                                    coverage_map,
                                    pc,
                                    &values,
                                    pc_id,
                                    pc_locations_tuple,
                                    locations_mask,
                                ) {
                                    mca.append_row(at_parameter, &pc, &values, pc_locations_tuple.1);
                                }

                                solver.pop_all(1);
                            }

                            if coverage_map.is_covered() {
                                return;
                            }
                        }

                        map_index += 1;
                        map_array >>= 1;
                    } else {
                        break 'sup_index;
                    }
                }
                debug_assert_eq!(map_index & BIT_MASK, 0);
            }
        }
    }
}

/// Filter the [CoverageMap]. Sets all disallowed interactions as covered.
pub unsafe fn filter_map<
    'a,
    ValueId: Id,
    ParameterId: Id,
    S: Solver<'a>,
    const STRENGTH: usize,
>(
    solver: &mut S,
    parameters: &UVec<ValueId>,
    at_parameter: usize,
    pc_list: &PCList<ParameterId, STRENGTH>,
    start: usize,
    end: usize,
    coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let mut map_index = 0;

    for pc in pc_list.pcs[start..end].iter() {
        let mut values = [ValueId::default(); STRENGTH - 1];
        let mut max_values = [ValueId::default(); STRENGTH - 1];
        for (max_value, parameter_id) in max_values.iter_mut().zip(pc.iter()) {
            solver.push_and_assert_eq(*parameter_id, ValueId::default());
            *max_value = parameters[parameter_id.as_usize()];
        }
        let value_choices = parameters[at_parameter];

        map_index += 1;
        let mut first_iteration = true;

        'value_loop: loop {
            for value in ValueId::default()..value_choices {
                if first_iteration {
                    first_iteration = false;
                    continue;
                }

                solver.push_and_assert_eq(ParameterId::from_usize(at_parameter), value);

                if !solver.check() {
                    coverage_map.set_index(map_index);
                }

                solver.pop(1);
                map_index += 1;
            }

            let mut value_index = STRENGTH - 2;
            values[value_index] += ValueId::from_usize(1);

            while 0 < value_index && values[value_index] == max_values[value_index] {
                values[value_index] = ValueId::default();
                values[value_index - 1] += ValueId::from_usize(1);
                value_index -= 1;
            }

            solver.pop((STRENGTH - value_index - 1) as u32);
            if 0 == value_index && values[0] == max_values[0] {
                break 'value_loop;
            }

            solver.push_and_assert_eq(pc[value_index], values[value_index]);

            while value_index < STRENGTH - 2 {
                value_index += 1;

                solver.push_and_assert_eq(pc[value_index], values[value_index]);
            }
        }
    }
}

/// The struct with the IPOG run method.
pub struct ConstrainedIPOG<
    'a,
    ValueId: Id,
    ParameterId: Id,
    S: Solver<'a>,
    HorizontalExtension: Extension<ValueId, ParameterId, STRENGTH>,
    VerticalExtension: Extension<ValueId, ParameterId, STRENGTH>,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,

    _solver: &'a PhantomData<S>,

    horizontal_extension: PhantomData<HorizontalExtension>,
    vertical_extension: PhantomData<VerticalExtension>,
}

impl<
    'a, ValueId: Id, ParameterId: Id, S: Solver<'a>,
    HorizontalExtension: Extension<ValueId, ParameterId, STRENGTH>,
    VerticalExtension: Extension<ValueId, ParameterId, STRENGTH>,
    const STRENGTH: usize> ConstrainedIPOG<'a, ValueId, ParameterId, S, HorizontalExtension, VerticalExtension, STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    /// Run the constrained version of IPOG.
    pub fn run(
        sut: &mut ConstrainedSUT<ValueId, ParameterId>,
        solver_init: &'a S::Init,
    ) -> MCA<ValueId> {
        let mut solver = sut.get_solver::<S>(&solver_init);
        let mut mca = MCA::<ValueId>::new_constrained::<ParameterId, S, STRENGTH>(
            &sut.sub_sut.parameters,
            &mut solver,
        );

        if STRENGTH == sut.sub_sut.parameters.len() {
            return mca;
        }

        let pc_list = sub_time_it!(
            PCList::<ParameterId, STRENGTH>::new(sut.sub_sut.parameters.len()),
            "PCList generation"
        );
        let mut coverage_map = CoverageMap::<ValueId, STRENGTH>::new(
            sut.sub_sut.parameters.clone(),
            &pc_list,
        );
        for at_parameter in STRENGTH..sut.sub_sut.parameters.len() {
            let pc_list_len = pc_list.sizes[at_parameter - STRENGTH];
            coverage_map.initialise(at_parameter);

            if cfg!(feature = "filter-map") {
                unsafe {
                    filter_map(
                        &mut solver,
                        &sut.sub_sut.parameters,
                        at_parameter,
                        &pc_list,
                        0,
                        pc_list_len,
                        &mut coverage_map,
                    );
                }
            }

            debug_assert!(mca.check_locations());

            unsafe {
                TimedExtension::<ValueId, ParameterId, HorizontalExtension, STRENGTH>::extend(
                    &mut solver,
                    &sut.sub_sut.parameters,
                    at_parameter,
                    &pc_list,
                    pc_list_len,
                    &mut mca,
                    &mut coverage_map,
                )
            };
            if !coverage_map.is_covered() {
                debug_assert!(mca.check_locations());
                unsafe {
                    TimedExtension::<ValueId, ParameterId, VerticalExtension, STRENGTH>::extend(
                        &mut solver,
                        &sut.sub_sut.parameters,
                        at_parameter,
                        &pc_list,
                        pc_list_len,
                        &mut mca,
                        &mut coverage_map,
                    );
                }
            }
        }
        mca
    }
}

#[cfg(all(test, feature = "sut/constraints-common"))]
mod test;
