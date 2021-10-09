// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module provides the [CoverageMap] used during each iteration of IPOG.

#![cfg_attr(test, feature(test))]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]
#![allow(soft_unstable)]

use std::cmp::max;

use common::{Number, u_vec, UVec};
use pc_list::PCList;

#[cfg(test)]
mod test_map;

#[cfg(test)]
mod bench_score;

// TODO make this a property of the CoverageMap (maybe make it private)
/// This the type of the elements of the map.
pub type BitArray = u64;

// TODO type of elements of the map should not be forced to be the same as the index
/// The mask used to get the index of the specific bit in the array. This is the [usize] version.
pub const BIT_MASK_U: usize = std::mem::size_of::<BitArray>() * 8 - 1;
/// The mask used to get the index of the specific bit in the array. This is the [BitArray] version.
pub const BIT_MASK: BitArray = BIT_MASK_U as BitArray;
/// The number of bits to shift to get the index of the element in the map.
pub const BIT_SHIFT: usize = BIT_MASK_U.count_ones() as usize;

/// The number of dont-cares for which the non-bitwise solution is used.
///
/// Used in [CoverageMap::get_high_score_masked_triple_sub] to switch between the three `get_high_score` implementations.
pub const DONT_CARES_FOR_NAIVE: u32 = 2;

/// Get the highest scoring value.
///
/// Ties are solved by selecting the least used value.
/// If a tie persists then the `previous_value` is used to determine the best scoring value "closest" to this value (incrementing and cycling).
#[inline]
pub unsafe fn get_highscore<ValueId: Number>(
    scores: &UVec<UVec<BitArray>>,
    uses: &UVec<usize>,
    mut previous_value: ValueId,
) -> ValueId {
    previous_value = (previous_value + ValueId::from_usize(1)) % ValueId::from_usize(scores.len());
    let mut high_score: usize = scores[previous_value.as_usize()].len();
    let mut high_use: usize = uses[previous_value.as_usize()];
    let mut high_value: ValueId = previous_value;

    // Start at previous_value + 1 and cycle through all values.
    for value in (previous_value + ValueId::from_usize(1)..ValueId::from_usize(scores.len()))
        .chain(ValueId::from_usize(0)..previous_value)
    {
        let value_score = scores[value.as_usize()].len();
        let value_use = uses[value.as_usize()];
        if high_score < value_score || (high_score == value_score && value_use < high_use) {
            high_score = value_score;
            high_value = value;
            high_use = value_use;
        }
    }

    high_value
}

/// Get the highest scoring value while skipping the blacklisted values.
#[inline]
pub unsafe fn get_highscore_blacklisted<ValueId: Number>(
    scores: &UVec<UVec<BitArray>>,
    uses: &UVec<usize>,
    previous_value: ValueId,
    blacklist: &UVec<bool>,
) -> ValueId {
    debug_assert_eq!(scores.len(), blacklist.len());
    debug_assert!((previous_value.as_usize()) < scores.len());
    debug_assert!(blacklist.iter().filter(|&p| !*p).count() > 1);
    debug_assert!(!blacklist[previous_value.as_usize()]);
    let mut high_score: usize = scores[previous_value.as_usize()].len();
    let mut high_use: usize = uses[previous_value.as_usize()];
    let mut high_value: ValueId = previous_value;

    for value in (previous_value + ValueId::from_usize(1)..ValueId::from_usize(scores.len()))
        .chain(ValueId::from_usize(0)..previous_value)
    {
        if !blacklist[value.as_usize()] {
            let value_score = scores[value.as_usize()].len();
            let value_use = uses[value.as_usize()];
            if high_score < value_score || (high_score == value_score && value_use < high_use) {
                high_score = value_score;
                high_value = value;
                high_use = value_use;
            }
        }
    }

    high_value
}

/// This is the Coverage Map used during the iterations of IPOG.
///
/// Use [CoverageMap::initialise] at the start of each iteration.
///
/// Check whether all interactions are covered using [CoverageMap::is_covered].
///
/// # Examples
/// ```
/// # use common::{u_vec, UVec};
/// # use cm::CoverageMap;
/// # use pc_list::PCList;
/// let at_parameter = 5;
/// let parameters = u_vec![4, 3, 3, 3, 3, 2, 2];
/// let pc_list = PCList::<u8, u8, 4>::new(parameters.len());
/// let mut coverage_map = CoverageMap::<u8, 4>::new(parameters, &pc_list);
///
/// coverage_map.initialise(at_parameter);
/// unsafe { coverage_map.set_zero_covered() };
/// ```
#[derive(Default, Clone)]
pub struct CoverageMap<ValueId: Number, const STRENGTH: usize>
    where [(); STRENGTH - 1]: {
    /// This is the collection of bit arrays.
    pub map: UVec<BitArray>,

    /// This vector contains the values used to calculate the indices.
    ///
    /// Each PC in the [PCList] has a row at the same index.
    /// The first element is the absolute offset for the PC.
    /// The next elements are the relative offsets for each value in the PC.
    /// See [CoverageMap::get_base_index] for more details.
    ///
    /// Will be generated once in [CoverageMap::new] and is reused throughout the generation.
    pub sizes: UVec<[BitArray; STRENGTH - 1]>,
    sizes_len: usize,
    all_sizes_len: UVec<usize>,

    /// The number of PCs left to cover.
    pub uncovered: usize,
    parameters: UVec<ValueId>,
    value_choices: ValueId,
}

impl<ValueId: Number, const STRENGTH: usize> CoverageMap<ValueId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Create a new [CoverageMap] for the provided parameters.
    ///
    /// It is assumed that the provided [PCList] is created using the same parameters.
    ///
    /// Memory allocation is performed in this method.
    /// Consequent calls to methods of the [CoverageMap] should require any new allocation.
    pub fn new<ParameterId: Number, LocationsType: Number>(
        parameters: UVec<ValueId>,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
    ) -> Self {
        let mut offset: BitArray = 0;
        let mut sizes = u_vec![[0; STRENGTH - 1]; pc_list.pcs.len() + 1];

        for (pc, sub_sizes) in pc_list.pcs.iter().zip(sizes.iter_mut()) {
            sub_sizes[0] = offset;
            let mut vec_size: BitArray = parameters[pc[STRENGTH - 2].as_usize()].as_usize() as BitArray;
            for pc_index in (0..STRENGTH - 2).rev() {
                sub_sizes[pc_index + 1] = vec_size;
                vec_size *= parameters[pc[pc_index].as_usize()].as_usize() as BitArray;
            }

            offset += vec_size;
        }

        sizes[pc_list.pcs.len()][0] = offset;

        let mut max_coverage_map: usize = 0;
        for (value_count, pc_list_len) in parameters.iter().skip(STRENGTH).zip(pc_list.sizes.iter()) {
            max_coverage_map = max(
                max_coverage_map,
                value_count.as_usize() * sizes[*pc_list_len][0].as_usize(),
            );
        }

        let mut map = u_vec![0; (max_coverage_map >> BIT_SHIFT) + 1];

        if cfg!(debug_assertions) {
            println!("Max coverage map: {} => map size: {}", max_coverage_map, map.len());
        }

        unsafe { map.set_len(0); }

        Self {
            map,
            sizes,
            sizes_len: 0,
            all_sizes_len: pc_list.sizes.clone(),
            uncovered: 0,
            parameters,
            value_choices: ValueId::default(),
        }
    }

    /// Initialise the [CoverageMap] for the (next) iteration of IPOG.
    pub fn initialise(&mut self, at_parameter: usize) {
        debug_assert!(at_parameter < self.parameters.len());
        self.value_choices = self.parameters[at_parameter];
        debug_assert!(at_parameter - STRENGTH < self.all_sizes_len.len());
        self.sizes_len = self.all_sizes_len[at_parameter - STRENGTH];
        debug_assert!(self.sizes_len < self.sizes.len());
        self.uncovered = self.sizes[self.sizes_len][0].as_usize() * self.value_choices.as_usize();

        let length = (self.uncovered >> BIT_SHIFT) + 1;

        debug_assert!(length <= self.map.capacity(), "{} <= {}; {}", length, self.map.capacity(), self.sizes_len);

        unsafe {
            self.map.as_mut_ptr().write_bytes(0, self.map.len());
            self.map.set_len(length);
        }
    }

    #[inline]
    unsafe fn get(&self, index: BitArray) -> bool {
        let map_index = index as usize >> BIT_SHIFT;
        debug_assert!(map_index < self.map.len());
        let bit_index = 1 << (index & BIT_MASK);
        self.map[map_index] & bit_index != 0
    }

    /// Returns true iff all PCs are covered.
    #[inline]
    pub fn is_covered(&self) -> bool {
        self.uncovered == 0
    }

    #[inline]
    unsafe fn add_scores(&self, scores: &mut UVec<UVec<BitArray>>, mut base_index: BitArray) {
        for score in scores.iter_mut().take(self.value_choices.as_usize()) {
            if !self.get(base_index) {
                score.push(base_index);
            }

            base_index += 1;
        }
    }

    /// Get the list of indices covered by each value if it where chosen.
    #[inline]
    pub unsafe fn get_high_score<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        row: &[ValueId],
        scores: &mut UVec<UVec<BitArray>>,
    ) {
        self.get_high_score_sub(pc_list, row, scores, 0, pc_list_len)
    }

    /// Get the list of indices covered by each value if it where chosen for the specified PCs.
    #[inline]
    pub unsafe fn get_high_score_sub<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        scores: &mut UVec<UVec<BitArray>>,
        start: usize,
        end: usize,
    ) {
        for pc_id in start..end {
            if let Some(base_index) = self.get_base_index(pc_id, pc_list, row) {
                self.add_scores(scores, base_index);
            }
        }
    }

    /// Get the list of indices covered by each value if it where chosen.
    ///
    /// This method will use the bit arrays to perform a slightly faster calculation of score.
    #[inline]
    pub fn get_high_score_masked<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        row: &[ValueId],
        dont_care_locations: LocationsType,
        no_dont_cares: LocationsType,
        scores: &mut UVec<UVec<BitArray>>,
    ) {
        self.get_high_score_masked_sub(pc_list, row, dont_care_locations, no_dont_cares, scores, 0, pc_list_len)
    }

    /// Get the list of indices covered by each value if it where chosen for the specified PCs.
    ///
    /// This method will use the bit arrays to perform a slightly faster calculation of score.
    #[inline]
    pub fn get_high_score_masked_sub<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        dont_care_locations: LocationsType,
        no_dont_cares: LocationsType,
        scores: &mut UVec<UVec<BitArray>>,
        start: usize,
        end: usize,
    ) {
        debug_assert!(((no_dont_cares << LocationsType::from_usize(1)) & dont_care_locations).any());
        if (no_dont_cares & dont_care_locations).none() {
            debug_assert_eq!(!no_dont_cares, dont_care_locations);
            unsafe { self.get_high_score_masked_unchecked_sub(pc_list, row, scores, start, end); }
        } else {
            debug_assert_ne!(!no_dont_cares, dont_care_locations);
            unsafe { self.get_high_score_masked_checked_sub(pc_list, row, dont_care_locations, scores, start, end); }
        }
    }

    /// Get the list of indices covered by each value if it where chosen.
    ///
    /// This method will use the bit arrays to perform a slightly faster calculation of score.
    /// If the number of dont-cares is equal to or lower than [DONT_CARES_FOR_NAIVE] then no bit arrays are used.
    #[inline]
    pub fn get_high_score_masked_triple<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        row: &[ValueId],
        dont_care_locations: LocationsType,
        no_dont_cares: LocationsType,
        scores: &mut UVec<UVec<BitArray>>,
    ) {
        self.get_high_score_masked_triple_sub(pc_list, row, dont_care_locations, no_dont_cares, scores, 0, pc_list_len)
    }

    /// Get the list of indices covered by each value if it where chosen for the specified PCs.
    ///
    /// This method will use the bit arrays to perform a slightly faster calculation of score.
    /// If the number of dont-cares is equal to or lower than [DONT_CARES_FOR_NAIVE] then no bit arrays are used.
    #[inline]
    pub fn get_high_score_masked_triple_sub<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        dont_care_locations: LocationsType,
        no_dont_cares: LocationsType,
        scores: &mut UVec<UVec<BitArray>>,
        start: usize,
        end: usize,
    ) {
        debug_assert_ne!((no_dont_cares << LocationsType::from_usize(1)) & dont_care_locations, LocationsType::default());
        let dont_care_count = (no_dont_cares & dont_care_locations).count_ones();
        if dont_care_count == 0 {
            debug_assert_eq!(!no_dont_cares, dont_care_locations);
            unsafe { self.get_high_score_masked_unchecked_sub(pc_list, row, scores, start, end); }
        } else if dont_care_count <= DONT_CARES_FOR_NAIVE {
            debug_assert_ne!(!no_dont_cares, dont_care_locations);
            debug_assert!((0..no_dont_cares.count_ones()).filter(|i| dont_care_locations.get(*i as usize)).count() <= DONT_CARES_FOR_NAIVE as usize);
            debug_assert!((0..no_dont_cares.count_ones()).filter(|i| dont_care_locations.get(*i as usize)).count() > 0);
            unsafe { self.get_high_score_sub(pc_list, row, scores, start, end); }
        } else {
            debug_assert!((0..no_dont_cares.count_ones()).filter(|i| dont_care_locations.get(*i as usize)).count() > DONT_CARES_FOR_NAIVE as usize);
            debug_assert_ne!(!no_dont_cares, dont_care_locations);
            unsafe { self.get_high_score_masked_checked_sub(pc_list, row, dont_care_locations, scores, start, end); }
        }
    }

    /// Get the list of indices covered by each value if it where chosen.
    ///
    /// This method will calculate the score while using the bitmasks to skip PCs with don't-cares
    #[inline]
    #[allow(dead_code)] // used in benchmarks
    pub unsafe fn get_high_score_masked_checked<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        row: &[ValueId],
        dont_care_locations: LocationsType,
        scores: &mut UVec<UVec<BitArray>>,
    ) {
        self.get_high_score_masked_checked_sub(pc_list, row, dont_care_locations, scores, 0, pc_list_len)
    }

    /// Get the list of indices covered by each value if it where chosen for the specified PCs.
    ///
    /// This method will calculate the score while using the bitmasks to skip PCs with don't-cares
    #[inline]
    pub unsafe fn get_high_score_masked_checked_sub<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        dont_care_locations: LocationsType,
        scores: &mut UVec<UVec<BitArray>>,
        start: usize,
        end: usize,
    ) {
        for tid in start..end {
            if (pc_list.locations[tid] & dont_care_locations).none() {
                self.add_scores(scores, self.get_base_index_unchecked(tid, pc_list, row));
            }
        }
    }

    /// Get the list of indices covered by each value if it where chosen.
    ///
    /// This method will calculate the score while skipping all dont-care checks. Use only if the row does not contain dont-cares.
    #[inline]
    #[allow(dead_code)] // used in benchmarks
    pub unsafe fn get_high_score_masked_unchecked<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        row: &[ValueId],
        scores: &mut UVec<UVec<BitArray>>,
    ) {
        self.get_high_score_masked_unchecked_sub(pc_list, row, scores, 0, pc_list_len)
    }

    /// Get the list of indices covered by each value if it where chosen for the specified PCs.
    ///
    /// This method will calculate the score while skipping all dont-care checks. Use only if the row does not contain dont-cares.
    #[inline]
    pub unsafe fn get_high_score_masked_unchecked_sub<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        scores: &mut UVec<UVec<BitArray>>,
        start: usize,
        end: usize,
    ) {
        for tid in start..end {
            self.add_scores(scores, self.get_base_index_unchecked(tid, pc_list, row));
        }
    }

    /// Get the list of indices covered by each of the specified values if it where chosen for the specified PCs.
    #[inline]
    pub unsafe fn get_high_score_sub_values_limited<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        masked_value_choices: &[ValueId],
        scores: &mut UVec<UVec<BitArray>>,
        start: usize,
        end: usize,
    ) {
        for pc_id in start..end {
            if let Some(base_index) = self.get_base_index(pc_id, pc_list, row) {
                for &value in masked_value_choices {
                    let index = base_index + value.as_usize() as BitArray;
                    if !self.get(base_index) {
                        scores[value.as_usize()].push(index);
                    }
                }
            }
        }
    }

    /// Get the previous value and set the index as covered.
    /// Does not decrease [CoverageMap::uncovered].
    unsafe fn get_and_set(&mut self, index: BitArray) -> bool {
        let map_index = index as usize >> BIT_SHIFT;
        debug_assert!(map_index < self.map.len());
        let bit_index = 1 << (index & BIT_MASK);

        let array = &mut self.map[map_index];
        if *array & bit_index == 0 {
            *array |= bit_index;
            false
        } else {
            true
        }
    }

    /// Set an index as covered. Assumes and expects the index not to be covered yet.
    /// Does not decrease [CoverageMap::uncovered].
    #[inline]
    pub unsafe fn set(&mut self, index: BitArray) {
        let array = &mut self.map[index as usize >> BIT_SHIFT];
        debug_assert_eq!(*array & (1 << (index & BIT_MASK)), 0);
        *array |= 1 << (index & BIT_MASK);
    }

    /// Sets the given index. If the index was already covered, then return `false`.
    /// Otherwise decrease [CoverageMap::uncovered] and return `true`.
    #[inline]
    pub unsafe fn set_index(&mut self, index: BitArray) -> bool {
        if !self.get_and_set(index) {
            self.uncovered -= 1;
            true
        } else {
            false
        }
    }

    /// Set all the indices as covered.
    /// Assumes all indices are not covered.
    pub unsafe fn set_indices(&mut self, indices: &UVec<BitArray>) {
        self.uncovered -= indices.len();
        self.set_indices_sub(indices)
    }

    /// Same as [CoverageMap::set_indices], but decreases the [CoverageMap::uncovered] by `indices.len() - filtered`.
    pub unsafe fn set_indices_updated(&mut self, indices: &UVec<BitArray>, filtered: usize) {
        self.uncovered -= indices.len() - filtered;
        self.set_indices_sub(indices)
    }

    /// Same as [CoverageMap::set_indices], but does not decrease the [CoverageMap::uncovered].
    pub unsafe fn set_indices_sub(&mut self, indices: &UVec<BitArray>) {
        for &index in indices.iter() {
            self.set(index)
        }
    }

    /// Set all the interactions in the row as covered.
    #[inline]
    pub unsafe fn set_covered_row_simple<ParameterId: Number, LocationsType: Number>(
        &mut self,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        pc_list_len: usize,
        row: &[ValueId],
    ) {
        self.set_covered_row_simple_sub(at_parameter, pc_list, row, 0, pc_list_len)
    }

    /// Set the interactions of the specified PCs in the row as covered.
    #[inline]
    pub unsafe fn set_covered_row_simple_sub<ParameterId: Number, LocationsType: Number>(
        &mut self,
        at_parameter: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
        start: usize,
        end: usize,
    ) {
        let value = row.get_unchecked(at_parameter).as_usize() as BitArray;
        for pc_id in start..end {
            if let Some(base_index) = self.get_base_index(pc_id, pc_list, row) {
                self.set_index(base_index + value);
            }
        }
    }

    /// Set all interactions with all zero values as covered.
    /// This method is used to handle the first row of the MCA (which is always an all zeros row).
    #[inline]
    pub unsafe fn set_zero_covered(&mut self) {
        self.uncovered -= self.sizes_len;
        let value_choices = self.value_choices.as_usize();
        for size in self.sizes.iter().take(self.sizes_len) {
            let index = *size.get_unchecked(0) as usize * value_choices;
            self.map[index as usize >> BIT_SHIFT] |= 1 << (index & BIT_MASK_U);
        }
    }

    /// Same as [CoverageMap::set_zero_covered], but only sets the specified range of PCs.
    /// Only reduces the [CoverageMap::uncovered] if `start` is zero.
    /// Assumes all PCs will be handled.
    #[inline]
    pub unsafe fn set_zero_covered_sub(&mut self, start: usize, end: usize) {
        if start == 0 {
            self.uncovered -= self.sizes_len;
        }
        let value_choices = self.value_choices.as_usize();
        for size in self.sizes[start..end].iter() {
            let index = *size.get_unchecked(0) as usize * value_choices;
            self.map[index >> BIT_SHIFT] |= 1 << (index & BIT_MASK_U);
        }
    }

    /// Set all covered indices to zero.
    /// Be sure to call [CoverageMap::set_indices_updated]
    #[inline]
    pub unsafe fn update_scores(&self, new_vec: &mut UVec<u64>) -> usize {
        let mut result = 0;

        for index in new_vec.iter_mut() {
            if self.get(*index) {
                *index = 0;
                result += 1;
            }
        }

        result
    }

    /// Get the base index for an interaction given a parameter_combination and a row.
    #[inline]
    pub unsafe fn get_base_index<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_id: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
    ) -> Option<BitArray> {
        let sizes = self.sizes[pc_id];
        let pc = pc_list.pcs[pc_id];

        let mut base_index = *sizes.get_unchecked(0);
        for index in 1..STRENGTH - 1 {
            let value = *row.get_unchecked(pc.get_unchecked(index - 1).as_usize());
            if value == ValueId::dont_care() {
                return None;
            }
            base_index += (value.as_usize() as BitArray) * *sizes.get_unchecked(index);
        }
        let value = *row.get_unchecked(pc.get_unchecked(STRENGTH - 2).as_usize());
        if value == ValueId::dont_care() {
            return None;
        }
        base_index += value.as_usize() as BitArray;

        base_index *= self.value_choices.as_usize() as BitArray;

        debug_assert!(base_index as usize >> BIT_SHIFT < self.map.len());
        Some(base_index)
    }

    /// Get the base index for an interaction given a parameter_combination and a row.
    /// This version does not check if the values are `don't-cares`.
    /// Use [CoverageMap::get_base_index] instead if you are not sure whether the values at the parameters used are `don't-cares`.
    #[inline]
    pub unsafe fn get_base_index_unchecked<ParameterId: Number, LocationsType: Number>(
        &self,
        pc_id: usize,
        pc_list: &PCList<ParameterId, LocationsType, STRENGTH>,
        row: &[ValueId],
    ) -> BitArray {
        let sizes = self.sizes[pc_id];
        let pc = pc_list.pcs[pc_id];

        let mut base_index = *sizes.get_unchecked(0);
        for index in 1..STRENGTH - 1 {
            base_index += (row.get_unchecked(pc.get_unchecked(index - 1).as_usize()).as_usize() as BitArray) * *sizes.get_unchecked(index);
        }
        base_index += row.get_unchecked(pc.get_unchecked(STRENGTH - 2).as_usize()).as_usize() as BitArray;

        base_index *= self.value_choices.as_usize() as BitArray;

        debug_assert!(base_index as usize >> BIT_SHIFT < self.map.len());
        base_index
    }
}
