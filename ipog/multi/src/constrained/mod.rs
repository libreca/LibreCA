// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the multithreaded IPOG implementation for [ConstrainedSUT]s.

use std::cmp::min;
use std::marker::PhantomData;
use std::ptr::replace;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;

use crossbeam::utils::Backoff;

use cm::{BitArray, CoverageMap};
use common::{Number, sub_time_it, u_vec, UVec};
use ipog_single::constrained::{Extension, HorizontalExtension, VerticalExtension};
use mca::{check_locations, MCA};
use sut::{ConstrainedSUT, Solver, SolverImpl};

use crate::{CACHE_MASK, IPOGData, Wrapper};
use crate::threads_common::{Response, Work};
use crate::unconstrained::threads::init_thread_pool;

unsafe fn update_scores<ValueId: Number, const STRENGTH: usize>(
    row_scores: &mut UVec<UVec<UVec<BitArray>>>,
    cm: &CoverageMap<ValueId, STRENGTH>,
    scores: &mut UVec<usize>,
) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    for thread_scores in row_scores.iter_mut() {
        for value in 0..scores.len() {
            let value_score = &mut scores[value];
            let thread_value_score = &mut thread_scores[value];
            let updated = cm.update_scores(thread_value_score);
            *value_score += thread_value_score.len() - updated;
        }
    }
}

#[inline]
unsafe fn get_highscore_blacklisted<ValueId: Number>(
    scores: &UVec<usize>,
    uses: &UVec<usize>,
    previous_value: ValueId,
    blacklist: &UVec<bool>,
) -> ValueId {
    debug_assert_eq!(scores.len(), blacklist.len());
    debug_assert!((previous_value.as_usize()) < scores.len());
    debug_assert!(blacklist.iter().filter(|&p| !*p).count() > 1);
    debug_assert!(!blacklist[previous_value.as_usize()]);
    let mut high_score: usize = scores[previous_value.as_usize()];
    let mut high_use: usize = uses[previous_value.as_usize()];
    let mut high_value: ValueId = previous_value;

    for value in (previous_value + ValueId::from_usize(1)..ValueId::from_usize(scores.len()))
        .chain(ValueId::from_usize(0)..previous_value)
    {
        if !blacklist[value.as_usize()] {
            let value_score = scores[value.as_usize()];
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

unsafe fn get_best_value<ValueId: Number, ParameterId: Number, const STRENGTH: usize>(
    solver: &mut SolverImpl,
    at_parameter: usize,
    mut previous_value: ValueId,
    value_choices: ValueId,
    scores: &mut UVec<usize>,
    blacklist: &mut UVec<bool>,
    uses: &mut UVec<usize>,
) -> Option<(ValueId, usize)> {
    for _ in 1..value_choices.as_usize() {
        // Try to fit the
        let value = get_highscore_blacklisted(&scores, &uses, previous_value, &blacklist);
        let score = scores[value.as_usize()];
        if score == 0 {
            return None;
        }

        solver.push_and_assert_eq(ParameterId::from_usize(at_parameter), value);
        let valid = solver.check();
        solver.pop(1);

        if valid {
            return Some((value, score));
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

    Some((value, scores[value.as_usize()]))
}


pub(crate) unsafe fn horizontal_extension_threaded<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize>(
    solver: &mut SolverImpl,
    senders: &[crossbeam::channel::Sender<Work<ValueId>>],
    _receivers: &[crossbeam::channel::Receiver<Response>],
    ipog_data: &mut IPOGData<ValueId, ParameterId, LocationsType, STRENGTH>,
    at_parameter: usize,
) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    ipog_data.at_row_main.store(0, SeqCst);
    ipog_data.cm.set_zero_covered();
    for sender in senders.iter() {
        sender.send(Work::NextParameter).unwrap();
    }

    let dont_care_mask = !LocationsType::bit(at_parameter);
    let mut previous_value = ValueId::default();
    let value_choices = ipog_data.parameters[at_parameter];
    let mut uses = u_vec![0; value_choices.as_usize()];
    uses[0] = 1;
    let mut scores = u_vec![0; value_choices.as_usize()];
    let mut blacklist = u_vec![false; value_choices.as_usize()];
    let mut worker_row = 0;
    let backoff = Backoff::new();

    for at_parameter_worker in ipog_data.at_parameter_worker.iter() {
        backoff.reset();
        while at_parameter_worker.load(SeqCst) != at_parameter {
            backoff.snooze();
        }
    }

    'row: for row_id in 1..ipog_data.mca.array.len() {
        ipog_data.at_row_main.store(row_id, SeqCst);
        for score in scores.iter_mut() { *score = 0; }
        for b in blacklist.iter_mut() { *b = false; }

        let row = ipog_data.mca.array[row_id].as_slice_mut();

        // Wait for all threads to finish the calculation.
        if worker_row <= row_id {
            worker_row = !0;

            for at_row_worker in ipog_data.at_row_worker.iter() {
                backoff.reset();
                let mut at_row_worker_raw = at_row_worker.load(SeqCst);
                while at_row_worker_raw <= row_id + 3 {
                    backoff.snooze();
                    at_row_worker_raw = at_row_worker.load(SeqCst);
                }
                worker_row = min(worker_row, at_row_worker_raw);
            }
        }

        let row_scores = &mut ipog_data.scores[row_id & CACHE_MASK];
        update_scores(row_scores, &ipog_data.cm, &mut scores);

        if scores.iter().all(|e| 0 == *e) {
            continue 'row;
        }

        solver.push_and_assert_row(&row[..at_parameter]);

        let fill_row = get_best_value::<ValueId, ParameterId, STRENGTH>(
            solver,
            at_parameter,
            previous_value,
            value_choices,
            &mut scores,
            &mut blacklist,
            &mut uses,
        );

        solver.pop(1); // Pop row

        if let Some((value, score)) = fill_row {
            row[at_parameter] = value;
            uses[value.as_usize()] += 1;
            previous_value = value;
            let dont_care_locations = &mut ipog_data.mca.dont_care_locations[row_id];
            *dont_care_locations &= dont_care_mask;
            debug_assert!(check_locations(row, *dont_care_locations));

            for thread_scores in row_scores.iter_mut() {
                ipog_data.cm.set_indices_sub(&mut thread_scores[value.as_usize()]);
            }

            ipog_data.cm.uncovered -= score;

            if ipog_data.cm.is_covered() {
                ipog_data.at_row_main.store(!0, SeqCst);
                return;
            }
        }
    }

    ipog_data.at_row_main.store(!0, SeqCst);
}


/// The struct with the IPOG run method.
pub struct ConstrainedMCIPOG<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize> {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
    locations_type: PhantomData<LocationsType>,
}

impl<ValueId: Number, ParameterId: Number, LocationsType: Number, const STRENGTH: usize> ConstrainedMCIPOG<ValueId, ParameterId, LocationsType, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Run the constrained version of IPOG.
    pub fn run(sut: Arc<ConstrainedSUT<ValueId, ParameterId>>, mut solver: SolverImpl) -> MCA<ValueId, LocationsType> {
        let mca = MCA::<ValueId, LocationsType>::new_constrained::<ParameterId, SolverImpl, STRENGTH>(
            &sut.sub_sut.parameters,
            &mut solver,
        );

        if STRENGTH == sut.sub_sut.parameters.len() {
            return mca;
        }

        let wrapper = Wrapper::<ValueId, ParameterId, LocationsType, STRENGTH>::new(sut.sub_sut.parameters.clone(), sut.count_constraints());
        unsafe { replace(&mut (*wrapper.data.get()).mca, mca); }
        let (senders, receivers) = sub_time_it!(init_thread_pool(wrapper.clone()), "T init");
        let ipog_data = unsafe { &mut *wrapper.data.get() };

        for at_parameter in STRENGTH..sut.sub_sut.parameters.len() {
            ipog_data.at_parameter_main.store(at_parameter, SeqCst);
            let pc_list_len = ipog_data.pc_list.sizes[at_parameter - STRENGTH];
            ipog_data.pc_list_len = pc_list_len;
            ipog_data.cm.initialise(at_parameter);

            if ipog_data.lower_limit <= pc_list_len {
                sub_time_it!(unsafe { horizontal_extension_threaded(&mut solver, &senders, &receivers, ipog_data, at_parameter) }, "HMulti  ");
            } else {
                sub_time_it!( unsafe { HorizontalExtension::extend(&mut solver, &ipog_data.parameters, at_parameter, &ipog_data.pc_list, pc_list_len, &mut ipog_data.mca, &mut ipog_data.cm) }, "HSingle");
            }

            if !ipog_data.cm.is_covered() {
                sub_time_it!( unsafe { VerticalExtension::extend(&mut solver, &sut.sub_sut.parameters, at_parameter, &ipog_data.pc_list, pc_list_len, &mut ipog_data.mca, &mut ipog_data.cm) }, "vertical");
            }

            if ipog_data.lower_limit <= pc_list_len {
                for receiver in receivers.iter() {
                    while receiver.recv().unwrap() != Response::Done {}
                }
            }
        }

        ipog_data.get_mca()
    }
}
