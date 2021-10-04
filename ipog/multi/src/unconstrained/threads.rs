// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the code run by the threads.

use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::thread;

use crossbeam::channel::{bounded, Receiver, Sender};
use crossbeam::utils::Backoff;

use common::Id;
use cm::BitArray;

use crate::{CACHE_MASK, IPOGData, MAX_HEAD_START, Wrapper};
use crate::threads_common::{CHANNEL_BOUNDS, cycling_split, Response, Work};

/// Fills the MCA using multiple threads.
#[cfg(feature = "threaded-fill")]
pub unsafe fn new_mca<ValueId: Id, ParameterId: Id, const STRENGTH: usize>(thread_id: usize, ipog_data: &mut IPOGData<ValueId, ParameterId, STRENGTH>) {
    let (mut start, end) = crate::threads_common::split(ipog_data.thread_count, thread_id, ipog_data.mca.array.len());
    if start == 0 {
        start = 1;
    }

    if start < end {
        let mut row = u_vec![ValueId::dont_care(); ipog_data.parameters.len()];

        {
            let mut index = start;
            for (&level, cell) in ipog_data.parameters.iter().take(STRENGTH).zip(row.iter_mut()) {
                let value = ValueId::from_usize(index) % level;
                *cell = value;
                index -= value.as_usize();
                index /= level.as_usize();
            }
        }

        let mut row_pointer = ipog_data.mca.array.as_mut_ptr().offset(start as isize);
        for _ in start..end - 1 {
            row_pointer.write(row.clone());
            row_pointer = row_pointer.offset(1);

            row[0] += ValueId::from_usize(1);
            let mut parameter_id = 0;
            while row[parameter_id] == ipog_data.parameters[parameter_id] {
                row[parameter_id] = ValueId::default();
                row[parameter_id + 1] += ValueId::from_usize(1);
                parameter_id += 1;
            }
        }
        row_pointer.write(row.clone());
    }
}

pub(crate) unsafe fn horizontal_extension_worker<ValueId: Id, ParameterId: Id, const STRENGTH: usize>(ipog_data: &mut IPOGData<ValueId, ParameterId, STRENGTH>, thread_id: usize) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let at_row_worker = &ipog_data.at_row_worker[thread_id];
    at_row_worker.store(0, SeqCst);
    let at_parameter = ipog_data.at_parameter_main.load(SeqCst);
    ipog_data.at_parameter_worker[thread_id].store(at_parameter, SeqCst);
    let pc_list_len = ipog_data.pc_list.sizes[at_parameter - STRENGTH];
    let value_choices: ValueId = ipog_data.parameters[at_parameter];
    let mut splits = cycling_split(ipog_data.thread_count, thread_id, pc_list_len);
    let backoff = Backoff::new();
    let pc_list = &ipog_data.pc_list;
    let no_dont_cares = !((!0) << at_parameter as BitArray);
    for row_scores in ipog_data.scores.iter_mut() {
        row_scores[thread_id].truncate(value_choices.as_usize());
    }

    for row_id in 1..ipog_data.mca.array.len() {
        at_row_worker.store(row_id, SeqCst);
        let scores = &mut ipog_data.scores[row_id & CACHE_MASK][thread_id];
        let row = ipog_data.mca.array[row_id].as_slice();
        let dont_care_locations = &ipog_data.mca.dont_care_locations[row_id];

        backoff.reset();
        let mut at_row_main = ipog_data.at_row_main.load(SeqCst);
        while at_row_main.saturating_add(MAX_HEAD_START) < row_id {
            backoff.snooze();
            at_row_main = ipog_data.at_row_main.load(SeqCst);
        }

        if at_row_main >= ipog_data.mca.array.len() {
            at_row_worker.store(!0, SeqCst);
            return;
        }

        for score in scores.iter_mut() {
            score.clear();
        }

        let (start, end) = splits.next().unwrap();
        ipog_data.cm.get_high_score_masked_triple_sub(pc_list, row, *dont_care_locations, no_dont_cares, scores, start, end);
    }

    at_row_worker.store(!0, SeqCst);
}


fn thread_main<ValueId: Id, ParameterId: Id, const STRENGTH: usize>(thread_id: usize, ipog_data: &mut IPOGData<ValueId, ParameterId, STRENGTH>, sender: Sender<Response>, receiver: Receiver<Work<ValueId>>) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let max_value_choices = *ipog_data.parameters.get(STRENGTH).unwrap_or(&ValueId::default());
    // let mut current_scores_indexes = u_vec![UVec::with_capacity(ipog_data.parameters.len() * STRENGTH * STRENGTH * 100); max_value_choices.as_usize()];
    // let mut previous_scores_indexes = current_scores_indexes.clone();
    ipog_data.reduction[thread_id].reserve(max_value_choices.as_usize());
    for _ in 0..max_value_choices.as_usize() {
        ipog_data.reduction[thread_id].push(0);
    }

    loop {
        match receiver.recv() {
            #[cfg(feature = "threaded-fill")]
            Ok(Work::FillMCA) => {
                unsafe { new_mca(thread_id, ipog_data); }
                sender.send(Response::Done).unwrap();
            }
            Ok(Work::NextParameter) => {
                unsafe { horizontal_extension_worker(ipog_data, thread_id); }
                sender.send(Response::Done).unwrap();
            }
            Ok(Work::SetCovered(_)) => { panic!("Did not expect cover message!"); }
            Ok(_) => {}
            Err(_) => { return; }
        }
    }
}


pub(crate) fn init_thread_pool<ValueId: Id, ParameterId: 'static + Id, const STRENGTH: usize>(ipog_data_arc: Arc<Wrapper<ValueId, ParameterId, STRENGTH>>) -> (Vec<Sender<Work<ValueId>>>, Vec<Receiver<Response>>) where [(); STRENGTH - 1]:, [(); STRENGTH - 2]: {
    let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
    let mut senders = Vec::with_capacity(ipog_data.thread_count);
    let mut receivers = Vec::with_capacity(ipog_data.thread_count);

    for thread_id in 0..ipog_data.thread_count {
        let (sender, local_receiver) = bounded(CHANNEL_BOUNDS);
        senders.push(sender);
        let (local_sender, receiver) = bounded(CHANNEL_BOUNDS);
        receivers.push(receiver);
        let local_ipog_data_arc = ipog_data_arc.clone();

        thread::spawn(move || {
            let local_ipog_data = unsafe { &mut *local_ipog_data_arc.data.get() };
            thread_main::<ValueId, ParameterId, STRENGTH>(thread_id, local_ipog_data, local_sender, local_receiver);
        });
    }

    (senders, receivers)
}
