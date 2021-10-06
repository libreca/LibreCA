// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate provides a multithreaded implementation of IPOG.
//!
//! # Features
//! The following feature is provided by this crate:
//!   * `no-cycle-split` Do not cycle the division of work between the worker threads.

#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![cfg_attr(test, feature(test))]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

#[cfg(test)]
extern crate test;

use std::cell::UnsafeCell;
use std::mem::replace;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use cm::{BitArray, CoverageMap};
use common::{Number, u_vec, UVec};
use mca::MCA;
use pc_list::PCList;

#[cfg(test)]
mod threads_common_test;
#[cfg(test)]
mod bench_comm;
#[cfg(test)]
mod bench_filter;
#[cfg(all(test, feature = "threaded-fill"))]
mod covering_array;

pub mod threads_common;
pub mod unconstrained;
pub mod constrained;


// TODO Bitwise operation for locations to parallelize the find_row without conflicts


pub(crate) const MAX_HEAD_START: usize = 31;
pub(crate) const CACHE_SIZE: usize = (MAX_HEAD_START + 1).next_power_of_two();
pub(crate) const CACHE_MASK: usize = CACHE_SIZE - 1;
pub(crate) const CONSTRAINTS_SWITCH: usize = 40;

/// This is the data passed to the new threads.
pub struct IPOGData<ValueId: Number, ParameterId: Number, const STRENGTH: usize> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// The number of worker threads that work on the solution.
    pub thread_count: usize,

    /// The lower limit when the multithreaded implementation is used instead of the single-threaded implementation.
    pub lower_limit: usize,

    /// Used to collect all results from the various threads.
    pub reduction: UVec<UVec<usize>>,

    /// The current parameter processed by the main thread.
    pub at_parameter_main: AtomicUsize,

    /// The current parameter processed by the worker threads.
    pub at_parameter_worker: UVec<AtomicUsize>,

    /// The current row processed by the main thread.
    pub at_row_main: AtomicUsize,

    /// The current row processed by the worker threads.
    pub at_row_worker: UVec<AtomicUsize>,

    /// The scores collected by the worker threads.
    pub scores: UVec<UVec<UVec<UVec<BitArray>>>>,

    /// The parameters in the SUT.
    pub parameters: UVec<ValueId>,

    /// The (resulting) MCA.
    pub mca: MCA<ValueId>,

    /// The list of PCs used during generation.
    pub pc_list: PCList<ParameterId, STRENGTH>,

    /// The current length of the list of PCs.
    pub pc_list_len: usize,

    /// The coverage-map used during generation.
    pub cm: CoverageMap<ValueId, STRENGTH>,
}

impl<ValueId: Number, ParameterId: Number, const STRENGTH: usize> IPOGData<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Create a new struct for the given parameters.
    pub fn new(parameters: UVec<ValueId>, constraints: usize) -> Self {
        let pc_list = PCList::new(parameters.len());
        let cm = CoverageMap::new(parameters.clone(), &pc_list);
        let thread_count = if CONSTRAINTS_SWITCH <= constraints && num_cpus::get_physical() != num_cpus::get() {
            num_cpus::get_physical()
        } else {
            num_cpus::get() - 1
        };
        println!("tc={}", thread_count);
        let mut at_parameter_worker = UVec::with_capacity(thread_count);
        let mut at_row_worker = UVec::with_capacity(thread_count);
        for _ in 0..thread_count {
            at_parameter_worker.push(AtomicUsize::new(0));
            at_row_worker.push(AtomicUsize::new(0));
        }

        let mut sub_scores = UVec::with_capacity(parameters[STRENGTH].as_usize());
        let mut last_level = ValueId::default();
        for (at_parameter, &level) in parameters.iter().enumerate().skip(STRENGTH).rev() {
            if last_level < level {
                let pc_list_len = pc_list.sizes[at_parameter - STRENGTH];
                for _ in last_level..level {
                    sub_scores.push(UVec::with_capacity(pc_list_len / thread_count + 2 * thread_count));
                }
                last_level = level;
            }
        }

        Self {
            thread_count,
            lower_limit: thread_count * 2,
            reduction: u_vec![UVec::with_capacity(10); thread_count],
            at_parameter_main: AtomicUsize::new(0),
            at_parameter_worker,
            at_row_main: AtomicUsize::new(0),
            at_row_worker,

            scores: u_vec![u_vec![sub_scores; thread_count]; CACHE_SIZE],

            parameters,
            mca: MCA::new_empty(),
            pc_list,
            pc_list_len: 0,
            cm,
        }
    }

    /// Replace the [MCA] with an emtpy one and return the old one.
    pub fn get_mca(&mut self) -> MCA<ValueId> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
        replace(&mut self.mca, MCA::new_empty())
    }
}

/// This struct wraps around the [IPOGData] struct.
///
/// It allows for concurrent writes to the data without any checks, so it is super unsafe.
/// Do not use this if you care about your sanity.
pub struct Wrapper<ValueId: Number, ParameterId: Number, const STRENGTH: usize> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    data: UnsafeCell<IPOGData<ValueId, ParameterId, STRENGTH>>,
}

// It is ''safe'' to move this to other threads...
unsafe impl<ValueId: Number, ParameterId: Number, const STRENGTH: usize> Sync for Wrapper<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {}

impl<ValueId: Number, ParameterId: Number, const STRENGTH: usize> Wrapper<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Create an [Arc], which wraps around this wrapper, which wraps around the [IPOGData].
    ///
    /// The argument is passed directly to [IPOGData::new] to construct a new instance of the data struct.
    pub fn new(parameters: UVec<ValueId>, constraints: usize) -> Arc<Self> {
        Arc::new(Self {
            data: UnsafeCell::new(IPOGData::new(parameters, constraints)),
        })
    }

    /// This method returns the [IPOGData] wrapped by this struct.
    pub unsafe fn get_data(&self) -> &mut IPOGData<ValueId, ParameterId, STRENGTH> {
        &mut *self.data.get()
    }
}
