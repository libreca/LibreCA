// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the multithreaded IPOG implementation for unconstrained SUTs.

use std::cmp::min;
use std::marker::PhantomData;
use std::sync::atomic::Ordering::SeqCst;

use crossbeam::utils::Backoff;

use cm::{BitArray, CoverageMap};
use common::{Id, sub_time_it, time_it, u_vec, UVec};
use ipog_single::unconstrained::{Extension, HorizontalExtension, VerticalExtension};
use mca::{check_locations, DontCareArray, MCA};
use sut::SUT;
use threads::init_thread_pool;

use crate::{CACHE_MASK, IPOGData, Wrapper};
use crate::threads_common::{Response, Work};

pub mod threads;


#[inline]
unsafe fn get_high_score_and_update<ValueId: Id, const STRENGTH: usize>(
    row_scores: &mut UVec<UVec<UVec<BitArray>>>,
    cm: &CoverageMap<ValueId, STRENGTH>,
    scores: &mut UVec<usize>, uses: &UVec<usize>,
    mut previous_value: ValueId,
) -> (ValueId, usize) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    for thread_scores in row_scores.iter_mut() {
        for value in 0..scores.len() {
            let value_score = &mut scores[value];
            let thread_value_score = &mut thread_scores[value];
            *value_score += thread_value_score.len();
        }
    }

    previous_value = (previous_value + ValueId::from_usize(1)) % ValueId::from_usize(scores.len());
    let score = &mut scores[previous_value.as_usize()];
    for thread_scores in row_scores.iter_mut() {
        *score -= cm.update_scores(&mut thread_scores[previous_value.as_usize()]);
    }

    let mut high_score: usize = *score;
    let mut high_use: usize = uses[previous_value.as_usize()];
    let mut high_value: ValueId = previous_value;

    for value in (previous_value + ValueId::from_usize(1)..ValueId::from_usize(scores.len())).chain(ValueId::from_usize(0)..previous_value) {
        let value_score = &mut scores[value.as_usize()];
        let value_use = uses[value.as_usize()];
        if high_score < *value_score || (high_score == *value_score && value_use < high_use) {
            for thread_scores in row_scores.iter_mut() {
                *value_score -= cm.update_scores(&mut thread_scores[value.as_usize()]);
            }

            if high_score < *value_score || (high_score == *value_score && value_use < high_use) {
                high_score = *value_score;
                high_value = value;
                high_use = value_use;
            }
        }
    }

    (high_value, high_score)
}


pub(crate) unsafe fn horizontal_extension_threaded<ValueId: Id, ParameterId: Id, const STRENGTH: usize>(
    senders: &[crossbeam::channel::Sender<Work<ValueId>>],
    _receivers: &[crossbeam::channel::Receiver<Response>],
    ipog_data: &mut IPOGData<ValueId, ParameterId, STRENGTH>,
    at_parameter: usize,
) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    ipog_data.at_row_main.store(0, SeqCst);
    ipog_data.cm.set_zero_covered();
    for sender in senders.iter() {
        sender.send(Work::NextParameter).unwrap();
    }

    let dont_care_mask: DontCareArray = !(1 << at_parameter as DontCareArray);
    let mut previous_value = ValueId::default();
    let value_choices = ipog_data.parameters[at_parameter];
    let mut uses = u_vec![0; value_choices.as_usize()];
    uses[0] = 1;
    let mut scores = u_vec![0; value_choices.as_usize()];
    let mut worker_row = 0;
    let backoff = Backoff::new();

    for at_parameter_worker in ipog_data.at_parameter_worker.iter() {
        backoff.reset();
        while at_parameter_worker.load(SeqCst) != at_parameter {
            backoff.snooze();
        }
    }

    for row_id in 1..ipog_data.mca.array.len() {
        ipog_data.at_row_main.store(row_id, SeqCst);
        for score in scores.iter_mut() { *score = 0; }
        // for score_update in score_updated.iter_mut() { *score_update = false; }

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

        // let (value, score) = get_high_score_and_update(row_scores, &ipog_data.cm, &mut scores, &mut score_updated, &uses, previous_value);
        let (value, score) = get_high_score_and_update(row_scores, &ipog_data.cm, &mut scores, &uses, previous_value);

        if score != 0 {
            row[at_parameter] = value;
            uses[value.as_usize()] += 1;
            previous_value = value;
            let dont_care_locations = &mut ipog_data.mca.dont_care_locations[row_id];
            *dont_care_locations &= dont_care_mask;
            debug_assert!(check_locations(row, *dont_care_locations));

            for thread_scores in row_scores.iter() {
                ipog_data.cm.set_indices_sub(&thread_scores[value.as_usize()]);
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

/// The toplevel of the IPOG method.
pub struct UnconstrainedMCIPOG<ValueId: Id, ParameterId: Id, const STRENGTH: usize> {
    value_id: PhantomData<ValueId>,
    parameter_id: PhantomData<ParameterId>,
}

impl<ValueId: Id, ParameterId: Id, const STRENGTH: usize> UnconstrainedMCIPOG<ValueId, ParameterId, STRENGTH> where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    /// Performs the IPOG algorithm using the specified extension types.
    pub fn run(sut: &SUT<ValueId, ParameterId>) -> MCA<ValueId> {
        if STRENGTH == sut.parameters.len() {
            return MCA::<ValueId>::new_unconstrained::<ParameterId, STRENGTH>(&sut.parameters);
        }

        let wrapper = Wrapper::<ValueId, ParameterId, STRENGTH>::new(sut.parameters.clone(), 0);
        let (senders, receivers) = time_it!(init_thread_pool(wrapper.clone()), "T init");
        let ipog_data = unsafe { wrapper.get_data() };

        ipog_data.mca = MCA::<ValueId>::new_unconstrained::<ParameterId, STRENGTH>(&sut.parameters);

        for at_parameter in STRENGTH..sut.parameters.len() {
            ipog_data.at_parameter_main.store(at_parameter, SeqCst);
            let pc_list_len = ipog_data.pc_list.sizes[at_parameter - STRENGTH];
            ipog_data.pc_list_len = pc_list_len;
            ipog_data.cm.initialise(at_parameter);

            if ipog_data.lower_limit <= pc_list_len {
                sub_time_it!(unsafe { horizontal_extension_threaded(&senders, &receivers, ipog_data, at_parameter) }, "HMulti  ");
            } else {
                sub_time_it!(unsafe { HorizontalExtension::extend(&ipog_data.parameters, at_parameter, &ipog_data.pc_list, pc_list_len, &mut ipog_data.mca, &mut ipog_data.cm) }, "HSingle ");
            }

            if !ipog_data.cm.is_covered() {
                sub_time_it!(unsafe { VerticalExtension::extend(&ipog_data.parameters, at_parameter, &ipog_data.pc_list, pc_list_len, &mut ipog_data.mca, &mut ipog_data.cm) }, "vertical");
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

#[cfg(test)]
pub mod bench_init;
#[cfg(test)]
pub mod bench_horizontal;
