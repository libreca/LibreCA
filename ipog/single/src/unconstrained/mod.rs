// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the IPOG algorithm for unconstrained [SUT]s.

use std::marker::PhantomData;

use cm::{BIT_MASK, BIT_SHIFT, BitArray, CoverageMap, get_highscore};
use common::{Number, sub_time_it, u_vec, UVec, ValueGenerator};
use mca::{check_locations, MCA};
use pc_list::PCList;
use sut::SUT;

/// This trait allows for the switching of various IPOG extension methods.
pub trait Extension<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Used for debugging purposes.
    const NAME: &'static str;

    /// Does the extension for the specified strength.
    unsafe fn extend(
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId, LocationsType>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    );
}

/// This extension method does nothing.
/// Used while debugging and for comparison with implementations that have unimplemented extensions.
pub struct NOOPExtension<const STRENGTH: usize>;

impl<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize>
Extension<ValueId, ParameterId, LocationsType, STRENGTH> for NOOPExtension<STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = "NOOP";

    unsafe fn extend(
        _parameters: &UVec<ValueId>,
        _at_parameter: usize,
        _pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        _pc_list_len: usize,
        _mca: &mut MCA<ValueId, LocationsType>,
        _coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {}
}

/// This extension prints the timing for the specified SubExtension.
pub struct TimedExtension<
    ValueId: Number,
    ParameterId: Number,
    LocationsType: Number,
    SubExtension: Extension<ValueId, ParameterId, LocationsType, STRENGTH>,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    sub_extension: PhantomData<SubExtension>,
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
    locations_type: PhantomData<LocationsType>,
}

impl<
    ValueId: Number,
    ParameterId: Number,
    LocationsType: Number,
    SubExtension: Extension<ValueId, ParameterId, LocationsType, STRENGTH>,
    const STRENGTH: usize,
> Extension<ValueId, ParameterId, LocationsType, STRENGTH>
for TimedExtension<ValueId, ParameterId, LocationsType, SubExtension, STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = SubExtension::NAME;

    unsafe fn extend(
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId, LocationsType>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {
        sub_time_it!(
            SubExtension::extend(
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

/// This horizontal extension will use bitwise operations to speed up the generation of the MCA.
pub struct HorizontalExtension<
    ValueId: Number,
    ParameterId: Number,
    LocationsType: Number,
    const STRENGTH: usize,
> {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
    locations_type: PhantomData<LocationsType>,
}

impl<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize>
Extension<ValueId, ParameterId, LocationsType, STRENGTH>
for HorizontalExtension<ValueId, ParameterId, LocationsType, STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = "B HE";

    unsafe fn extend(
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId, LocationsType>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {
        debug_assert!(!mca.dont_care_locations.is_empty());

        let dont_care_mask = !LocationsType::bit(at_parameter);
        let no_dont_cares = LocationsType::mask_low(at_parameter);
        let value_choices = parameters[at_parameter];
        let mut scores = u_vec![UVec::with_capacity(pc_list_len); value_choices.as_usize()];
        let mut previous_value: ValueId = ValueId::default();
        let mut uses = u_vec![0; value_choices.as_usize()];
        uses[0] = 1;
        coverage_map.set_zero_covered();

        for row_id in 1..mca.array.len() {
            let row = mca.array[row_id].as_slice_mut();
            let dont_care_locations = &mut mca.dont_care_locations[row_id];

            for score in scores.iter_mut() {
                score.clear();
            }

            // TODO Move to CM
            if cfg!(feature="score-double") {
                coverage_map.get_high_score_masked(
                    pc_list,
                    pc_list_len,
                    row,
                    *dont_care_locations,
                    no_dont_cares,
                    &mut scores,
                );
            } else if cfg!(feature="score-single") {
                coverage_map.get_high_score(
                    pc_list,
                    pc_list_len,
                    row,
                    &mut scores,
                );
            } else {
                coverage_map.get_high_score_masked_triple(
                    pc_list,
                    pc_list_len,
                    row,
                    *dont_care_locations,
                    no_dont_cares,
                    &mut scores,
                );
            }

            let value: ValueId = get_highscore(&scores, &uses, previous_value);

            if !scores[value.as_usize()].is_empty() {
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

/// This vertical extension will use bitwise operations to speed up the generation of the MCA.
/// It will keep track of the number of rows without dont_care values at the start of the MCA and skip them in each iteration.
pub struct VerticalExtension<
    ValueId: Number,
    ParameterId: Number,
    LocationsType: Number,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
    locations_type: PhantomData<LocationsType>,
}

impl<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize>
VerticalExtension<ValueId, ParameterId, LocationsType, STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    #[inline]
    unsafe fn pc_fits_row(
        at_parameter: usize,
        pc: &[ParameterId; STRENGTH - 1],
        values: &[ValueId; STRENGTH],
        pc_locations_tuple: &(LocationsType, LocationsType),
        row: &mut [ValueId],
        dont_care_locations: LocationsType,
    ) -> bool {
        let shared_dont_cares = dont_care_locations & pc_locations_tuple.0;
        if shared_dont_cares.none() {
            return false;
        }

        if shared_dont_cares != pc_locations_tuple.0 {
            for (&parameter_id, &value) in pc.iter().zip(values.iter()) {
                if *row.get_unchecked(parameter_id.as_usize()) != value
                    && *row.get_unchecked(parameter_id.as_usize()) != ValueId::dont_care()
                {
                    // If the interaction does not fit go to the next row
                    return false;
                }
            }
        }

        let last_value_row = row.get_unchecked_mut(at_parameter);
        let last_value_pc = *values.get_unchecked(STRENGTH - 1);
        if *last_value_row == ValueId::dont_care() {
            *last_value_row = last_value_pc;
        } else if *last_value_row != last_value_pc {
            return false;
        }

        true
    }

    /// Try to fit the interaction in the existing rows by setting dont_care values.
    #[inline]
    unsafe fn fit_in_row(
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId, LocationsType>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
        pc: &[ParameterId; STRENGTH - 1],
        values: &[ValueId; STRENGTH],
        pc_id: usize,
        pc_locations_tuple: &(LocationsType, LocationsType),
        dont_care_mask: LocationsType,
    ) -> bool {
        // iterate over all rows of the MCA (but skip the first ones)
        for (ve_index, &row_id) in mca.vertical_extension_rows.iter().enumerate() {
            let row = mca.array[row_id].as_slice_mut();
            let dont_care_locations = &mut mca.dont_care_locations[row_id];

            if Self::pc_fits_row(
                at_parameter,
                pc,
                values,
                pc_locations_tuple,
                row,
                *dont_care_locations,
            ) {
                // Interaction fits in the row, so fill the values in the row
                for (&parameter_id, &value) in pc.iter().zip(values.iter()) {
                    *row.get_unchecked_mut(parameter_id.as_usize()) = value;
                }

                *dont_care_locations &= pc_locations_tuple.1;

                if (*dont_care_locations & dont_care_mask).none() {
                    mca.vertical_extension_rows.remove(ve_index);
                }

                debug_assert!(check_locations(row, *dont_care_locations));

                coverage_map.set_covered_row_simple_sub(
                    at_parameter,
                    &pc_list,
                    row,
                    pc_id + 1, // TODO max(pc_id + 1, pc_list.sizes[first_parameter_changed - STRENGTH])
                    pc_list_len,
                );

                // Done with this interaction, so stop iterating
                return true;
            }
        }

        false
    }
}

impl<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize>
Extension<ValueId, ParameterId, LocationsType, STRENGTH>
for VerticalExtension<ValueId, ParameterId, LocationsType, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]:
{
    const NAME: &'static str = "B VE";

    unsafe fn extend(
        parameters: &UVec<ValueId>,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        mca: &mut MCA<ValueId, LocationsType>,
        coverage_map: &mut CoverageMap<ValueId, STRENGTH>,
    ) {
        debug_assert!(
            ((BIT_MASK + 1) as usize) < (ValueId::dont_care().as_usize()),
            "Bitarray and ValueId are incompatible."
        );
        let value_choices = parameters[at_parameter].as_usize() as BitArray;

        let dont_care_mask = mca.set_vertical_extension_rows(at_parameter);

        debug_assert!(mca.check_all(at_parameter));

        let parameter_mask = LocationsType::bit(at_parameter);

        // Ignore the fact that pc_list should be bounded by the pc_list_len, because the uncovered interactions will be covered before getting to the out of bound PCs.
        for (pc_id, (pc, dont_care_locations)) in pc_list
            .pcs
            .iter()
            .zip(pc_list.locations.iter())
            .enumerate()
        {
            let mut values = [ValueId::default(); STRENGTH];
            let value_generator = ValueGenerator::<ValueId, STRENGTH>::new(
                &parameters,
                at_parameter,
                pc,
            );
            let mut map_index = (coverage_map.sizes[pc_id][0] * value_choices) + 1;
            let mut pc_locations_option: Option<(LocationsType, LocationsType)> = None;

            'sup_index: loop {
                let map_sub_index = map_index & BIT_MASK;
                let mut map_array = coverage_map.map[map_index as usize >> BIT_SHIFT] >> map_sub_index;

                // Skip block if the block is covered
                if map_array == BitArray::max_value() {
                    if value_generator
                        .skip_array(&mut values, ValueId::from_usize(BIT_MASK as usize + 1))
                    {
                        map_index += BIT_MASK + 1;
                        continue 'sup_index;
                    } else {
                        break 'sup_index;
                    }
                }

                // Loop through every bit in the block
                for _ in map_sub_index..=BIT_MASK {
                    if value_generator.next_array(&mut values) {
                        // Updates the values and checks if we are done
                        if map_array & 1 == 0 {
                            // Check if values need covering
                            coverage_map.uncovered -= 1;

                            let pc_locations_tuple = pc_locations_option.get_or_insert_with(|| {
                                (*dont_care_locations, !(*dont_care_locations | parameter_mask))
                            });

                            if !Self::fit_in_row(
                                at_parameter,
                                pc_list,
                                pc_list_len,
                                mca,
                                coverage_map,
                                pc,
                                &values,
                                pc_id,
                                pc_locations_tuple,
                                dont_care_mask,
                            ) {
                                mca.append_row(at_parameter, &pc, &values, pc_locations_tuple.1);
                            }

                            if coverage_map.is_covered() {
                                return;
                            }
                        }

                        map_index += 1;
                        map_array >>= 1;
                    } else {
                        // According to the value generator we are done, so break:
                        break 'sup_index;
                    }
                }

                debug_assert_eq!(map_index & BIT_MASK, 0);
            }
        }
    }
}

/// The toplevel of the IPOG method.
pub struct UnconstrainedIPOG<
    ValueId: Number,
    ParameterId: Number,
    LocationsType: Number,
    HorizontalExtension: Extension<ValueId, ParameterId, LocationsType, STRENGTH>,
    VerticalExtension: Extension<ValueId, ParameterId, LocationsType, STRENGTH>,
    const STRENGTH: usize,
> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
    locations_type: PhantomData<LocationsType>,

    horizontal_extension: PhantomData<HorizontalExtension>,
    vertical_extension: PhantomData<VerticalExtension>,
}

impl<ValueId: Number, ParameterId: Number, LocationsType: Number, HorizontalExtension: Extension<ValueId, ParameterId, LocationsType, STRENGTH>, VerticalExtension: Extension<ValueId, ParameterId, LocationsType, STRENGTH>, const STRENGTH: usize>
UnconstrainedIPOG<ValueId, ParameterId, LocationsType, HorizontalExtension, VerticalExtension, STRENGTH>
    where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Performs the IPOG algorithm using the specified extension types.
    pub fn run(sut: &mut SUT<ValueId, ParameterId>) -> MCA<ValueId, LocationsType> {
        let mut mca = MCA::<ValueId, LocationsType>::new_unconstrained::<ParameterId, STRENGTH>(&sut.parameters);

        if cfg!(debug_assertions) {
            println!("Initial: {:?}", mca.array.len());
        }

        if STRENGTH == sut.parameters.len() {
            return mca;
        }

        let pc_list = sub_time_it!(PCList::<ParameterId, LocationsType, STRENGTH>::new(sut.parameters.len()), "PCList generation");
        let mut coverage_map = CoverageMap::<ValueId, STRENGTH>::new(sut.parameters.clone(), &pc_list);

        for at_parameter in STRENGTH..sut.parameters.len() {
            let pc_list_len = pc_list.sizes[at_parameter - STRENGTH];
            coverage_map.initialise(at_parameter);

            if cfg!(debug_assertions) {
                println!("Uncovered: {:?}", coverage_map.uncovered);
            }

            debug_assert!(mca.check_locations());

            unsafe {
                TimedExtension::<ValueId, ParameterId, LocationsType, HorizontalExtension, STRENGTH>::extend(
                    &sut.parameters,
                    at_parameter,
                    &pc_list,
                    pc_list_len,
                    &mut mca,
                    &mut coverage_map,
                );
            }

            if !coverage_map.is_covered() {
                debug_assert!(mca.check_locations());
                unsafe {
                    TimedExtension::<ValueId, ParameterId, LocationsType, VerticalExtension, STRENGTH>::extend(
                        &sut.parameters,
                        at_parameter,
                        &pc_list,
                        pc_list_len,
                        &mut mca,
                        &mut coverage_map,
                    );
                }
                debug_assert!(mca.check_all(at_parameter));
            }
        }
        mca
    }
}

#[cfg(test)]
mod test;
