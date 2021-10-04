// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::thread;

use crossbeam::channel::{bounded, Receiver, RecvError, Sender};
use ipog_single::coverage_map::{BIT_MASK, BIT_MASK_U, BIT_SHIFT};
use ipog_single::value_generator::ValueGenerator;
use sut::{ConstrainedSUT, Id, Solver, SolverImpl};

use crate::{IPOGData, Wrapper};
use crate::threads_common::{CHANNEL_BOUNDS, filter_scores, Response, split, Work};

const CONSTRAINT_THREAD_BUFFER_SIZE: usize = 2;
const CONSTRAINT_BUFFER_SIZE: usize = 16;
const CONSTRAINT_CHANNEL_SIZE: usize = CONSTRAINT_BUFFER_SIZE / 2;
const CONSTRAINT_BUFFER_MASK: usize = CONSTRAINT_BUFFER_SIZE - 1;

struct ConstraintsMessage<ValueId: Id> {
    at_parameter: usize,
    row_id: usize,
    value_id: ValueId,
}

#[cfg(feature = "threaded-fill")]
pub fn new_mca<ValueId: Id, ParameterId: Id,  const STRENGTH: usize>(_thread_id: usize, _parameters: &[ValueId], _thread_count: usize, _mca: &mut ipog_single::MCA<ValueId>) {
    unimplemented!("Multithreaded MCA creation not implemented (yet).");
    /*let (mut start, end) = split(thread_count, thread_id, mca.array.len());
    if start == 0 {
        start = 1;
    }

    if start < end {
        let mut index = start;
        for parameter_id in (0..STRENGTH).rev() {
            let value = index % parameters[parameter_id];
            mca.array[start][parameter_id] = value;
            index -= value;
            index /= parameters[parameter_id];
        }

        for index in start + 1..end {
            let mut carry = 1;
            for parameter_id in (0..STRENGTH).rev() {
                let value = mca.array[index - 1][parameter_id] + carry;
                if value == parameters[parameter_id] {
                    carry = 1;
                    mca.array[index][parameter_id] = 0;
                } else {
                    carry = 0;
                    mca.array[index][parameter_id] = value;
                }
            }
        }
    }

    POOL_DATA.barrier.wait();*/
}

pub struct ConstraintResults<ValueId: Id> {
    results: UnsafeCell<[Vec<ValueId>; CONSTRAINT_BUFFER_SIZE]>,
    result_ids: UnsafeCell<[usize; CONSTRAINT_BUFFER_SIZE]>,
}

unsafe impl<ValueId: Id> Sync for ConstraintResults<ValueId> {}

fn thread_constraints_sub<ValueId: Id, ParameterId: Id,  const STRENGTH: usize>(sut: Arc<ConstrainedSUT<ValueId, ParameterId>>, ipog_data_arc: Arc<Wrapper<ValueId, ParameterId, S_STRENGTH, STRENGTH>>, thread_id: usize, thread_count: usize, sender: Sender<bool>, receiver: Receiver<ConstraintsMessage<ValueId>>, covered: Arc<AtomicUsize>) {
    let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
    let solver_init = SolverImpl::default_init();
    let mut solver = SolverImpl::new(&sut, &solver_init);

    for message in receiver {
        if message.row_id == 0 {
            let (start, end) = split(thread_count, thread_id, ipog_data.pc_list_len);
            for (pc_id, pc) in ipog_data.pc_list.pcs[start..end].iter().enumerate() {
                let mut values = [ValueId::default(); STRENGTH];
                let value_generator = ValueGenerator::new(&sut.sub_sut.parameters, message.at_parameter, pc);
                let mut map_index = ipog_data.cm.sizes[pc_id][0] + 1;

                'sup_index: loop {
                    let map_sub_index = map_index & BIT_MASK;
                    let mut map_array = ipog_data.cm.map[map_index as usize >> BIT_SHIFT] >> map_sub_index;

                    if map_array == !0 {
                        if value_generator.skip_array(&mut values, ValueId::from_usize(BIT_MASK_U + 1)) {
                            map_index += BIT_MASK + 1;
                            continue 'sup_index;
                        } else {
                            break 'sup_index;
                        }
                    }

                    for index_shift in map_sub_index..=BIT_MASK {
                        if value_generator.next_array(&mut values) {
                            if map_array & 1 == 0 {
                                solver.push_and_assert_pc(pc, message.at_parameter, &values);
                                if !solver.check_and_pop_all(1) {
                                    ipog_data.cm.map[map_index as usize >> BIT_SHIFT] |= 1 << index_shift;
                                    covered.fetch_add(1, Relaxed);
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
            sender.send(true).unwrap();
        } else {
            solver.push_and_assert_row(&ipog_data.mca.array[message.row_id][0..message.at_parameter]);
            solver.push_and_assert_eq(message.at_parameter, message.value_id.as_usize());
            sender.send(solver.check()).unwrap();
            solver.pop_all(2);
        }
    }
}

#[cfg_attr(not(feature = "filter-map"), allow(unused_variables))]
fn thread_constraints<ValueId: Id, ParameterId: Id,  const STRENGTH: usize>(sut: Arc<ConstrainedSUT<ValueId, ParameterId>>, ipog_data_arc: Arc<Wrapper<ValueId, ParameterId, S_STRENGTH, STRENGTH>>, thread_id: usize, senders: Vec<Sender<bool>>, main_sender: Sender<()>, receiver: Receiver<Work<ValueId>>, constraint_data: Arc<ConstraintResults<ValueId>>, thread_count: usize) {
    let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
    let results: &mut [Vec<ValueId>; CONSTRAINT_BUFFER_SIZE] = unsafe { &mut *constraint_data.results.get() };
    let capacity = ipog_data.parameters[STRENGTH].as_usize();
    for result in results.iter_mut() {
        result.reserve(capacity);
    }
    let result_ids: &mut [usize; CONSTRAINT_BUFFER_SIZE] = unsafe { &mut *constraint_data.result_ids.get() };

    let mut sub_senders = Vec::with_capacity(capacity);
    let mut sub_receivers = Vec::with_capacity(capacity);
    let covered = Arc::new(AtomicUsize::new(0));

    for thread_id in 0..thread_count {
        let local_sut = sut.clone();
        let (sender, local_receiver) = bounded(CONSTRAINT_THREAD_BUFFER_SIZE + 1);
        sub_senders.push(sender);
        let (local_sender, receiver) = bounded(CHANNEL_BOUNDS);
        sub_receivers.push(receiver);
        let local_ipog_data_arc = ipog_data_arc.clone();
        let local_covered = covered.clone();

        thread::spawn(move || {
            thread_constraints_sub::<ValueId, ParameterId, S_STRENGTH, STRENGTH>(local_sut, local_ipog_data_arc, thread_id, thread_count, local_sender, local_receiver, local_covered);
        });
    }

    let mut sub_sender_cycle = sub_senders.iter().cycle();
    let mut sub_receiver_cycle = sub_receivers.iter().cycle();

    loop {
        match receiver.recv() {
            #[cfg(feature = "threaded-fill")]
            Ok(Work::FillMCA) => new_mca::<ValueId, ParameterId, S_STRENGTH, STRENGTH>(thread_id, &ipog_data.parameters, thread_count + 1, &mut ipog_data.mca),
            Ok(Work::NextParameter) => {
                for sender in senders.iter() {
                    sender.send(true).unwrap();
                }
                let at_parameter = ipog_data.at_parameter_main.load(SeqCst);
                let value_choices = ipog_data.parameters[at_parameter];

                let row_buf = thread_count * CONSTRAINT_THREAD_BUFFER_SIZE / value_choices.as_usize() + 1;

                for row_id in 1..row_buf {
                    for value_id in ValueId::default()..value_choices {
                        sub_sender_cycle.next().unwrap().send(ConstraintsMessage { at_parameter, row_id, value_id }).unwrap();
                    }
                }

                for (row_id, buf_row_id) in (1..ipog_data.mca.array.len()).zip(row_buf..) {
                    if buf_row_id < ipog_data.mca.array.len() {
                        for value_id in ValueId::default()..value_choices {
                            sub_sender_cycle.next().unwrap().send(ConstraintsMessage { at_parameter, row_id: buf_row_id, value_id }).unwrap();
                        }
                    }

                    let valid_values: &mut Vec<ValueId> = &mut results[row_id & CONSTRAINT_BUFFER_MASK];
                    valid_values.clear();

                    for value in ValueId::default()..value_choices {
                        if sub_receiver_cycle.next().unwrap().recv().unwrap() {
                            valid_values.push(value);
                        }
                    }

                    result_ids[row_id & CONSTRAINT_BUFFER_MASK] = row_id;

                    for sender in senders.iter() {
                        sender.send(false).unwrap();
                    }

                    match receiver.try_recv() {
                        Ok(Work::Covered) => {
                            for _ in 1..row_buf {
                                sub_receiver_cycle.next().unwrap().recv().unwrap();
                            }
                            break;
                        }
                        Ok(work) => { panic!("Unexpected: {:?}", work); }
                        Err(_) => {}
                    }
                }
            }
            #[cfg(feature = "filter-map")]
            Ok(Work::Filter) => {
                covered.store(0, SeqCst);
                let at_parameter: usize = ipog_data.at_parameter.load(SeqCst);
                for sender in sub_senders.iter() {
                    sender.send(ConstraintsMessage { at_parameter, row_id: 0, value_id: ValueId::default() }).unwrap();
                }

                for receiver in sub_receivers.iter() {
                    receiver.recv().unwrap();
                }
                ipog_data.cm.uncovered -= covered.load(SeqCst);
                main_sender.send(()).unwrap();
            }
            Ok(Work::SetCovered(_)) => panic!("Unexpected set cover for constraint thread."),
            Ok(Work::Covered) => {}
            Err(_) => { return; }
        }
    }
}


pub(crate) struct ThreadMain<'a, ValueId: Id, ParameterId: Id,  const STRENGTH: usize> {
    thread_id: usize,
    sender: Sender<Response>,
    receiver: Receiver<Work<ValueId>>,
    constraint_receiver: Receiver<bool>,
    thread_count: usize,
    ipog_data: &'a mut IPOGData<ValueId, ParameterId, S_STRENGTH, STRENGTH>,
    current_scores_indexes: Vec<Vec<u64>>,
    previous_scores_indexes: Vec<Vec<u64>>,
    results: &'a [Vec<ValueId>; CONSTRAINT_BUFFER_SIZE],
    result_ids: &'a [usize; CONSTRAINT_BUFFER_SIZE],
}

impl<ValueId: Id, ParameterId: Id,  const STRENGTH: usize> ThreadMain<'_, ValueId, ParameterId, S_STRENGTH, STRENGTH> {
    pub(crate) fn new(thread_id: usize, ipog_data_arc: Arc<Wrapper<ValueId, ParameterId, S_STRENGTH, STRENGTH>>, sender: Sender<Response>, receiver: Receiver<Work<ValueId>>, constraint_receiver: Receiver<bool>, constraint_data: Arc<ConstraintResults<ValueId>>, thread_count: usize) -> Self {
        let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
        let max_value_choices = *ipog_data.parameters.get(STRENGTH).unwrap_or(&ValueId::default());
        let current_scores_indexes = vec![Vec::with_capacity(ipog_data.parameters.len() * STRENGTH * STRENGTH * 100); max_value_choices.as_usize()];

        ipog_data.reduction[thread_id].reserve(max_value_choices.as_usize());
        for _ in ValueId::default()..max_value_choices {
            ipog_data.reduction[thread_id].push(0);
        }

        Self {
            thread_id,
            sender,
            receiver,
            constraint_receiver,
            thread_count,
            ipog_data,
            current_scores_indexes: current_scores_indexes.clone(),
            previous_scores_indexes: current_scores_indexes,
            results: unsafe { &*constraint_data.results.get() },
            result_ids: unsafe { &*constraint_data.result_ids.get() },
        }
    }

    fn thread_high_score(&mut self, expect_result: bool, row_id: &mut usize, value_choices: ValueId, start: usize, end: usize) -> (bool, bool) {
        if *row_id >= self.ipog_data.mca.array.len() {
            if expect_result {
                match self.receiver.recv() {
                    Ok(Work::Covered) => {}
                    Ok(Work::SetCovered(value)) => { unsafe { self.ipog_data.cm.set_indices_sub(&self.previous_scores_indexes[value.as_usize()]); } }
                    Ok(work) => { panic!("Unexpected: {:?}", work); }
                    Err(RecvError) => { panic!("Unexpected disconnect"); }
                }
            }
            return (true, false);
        }

        for scores_indexes in self.current_scores_indexes.iter_mut() {
            scores_indexes.clear();
        }

        self.constraint_receiver.recv().unwrap();

        unsafe { self.ipog_data.cm.get_high_score_sub_values_limited(&self.ipog_data.pc_list, &self.ipog_data.mca.array[*row_id], &self.results[*row_id & CONSTRAINT_BUFFER_MASK], &mut self.current_scores_indexes, start, end); }

        debug_assert_eq!(self.result_ids[*row_id & CONSTRAINT_BUFFER_MASK], *row_id);

        let mut is_covering = !self.current_scores_indexes.iter().all(|a| a.is_empty());

        if expect_result {
            match self.receiver.recv() {
                Ok(Work::Covered) => { return (true, false); }
                Ok(Work::SetCovered(value)) => {
                    unsafe { self.ipog_data.cm.set_indices_sub(&self.previous_scores_indexes[value.as_usize()]); }

                    if is_covering {
                        is_covering = false;
                        for index in 0..value_choices.as_usize() {
                            if index == value.as_usize() {
                                let deleted = filter_scores(&mut self.current_scores_indexes[index], &self.previous_scores_indexes[index]);
                                let score = self.current_scores_indexes[index].len() - deleted;
                                is_covering |= 0 < score;
                                self.ipog_data.reduction[self.thread_id][index] = score;
                            } else {
                                is_covering |= !self.current_scores_indexes[index].is_empty();
                                self.ipog_data.reduction[self.thread_id][index] = self.current_scores_indexes[index].len();
                            }
                        }
                    }
                }
                Ok(work) => { panic!("Unexpected: {:?}", work); }
                Err(RecvError) => { panic!("Unexpected disconnect"); }
            }
        } else {
            match self.receiver.try_recv() {
                Ok(Work::Covered) => { return (true, false); }
                Ok(work) => { panic!("Unexpected: {:?}", work); }
                Err(_) => {}
            }

            if is_covering {
                for index in 0..value_choices.as_usize() {
                    self.ipog_data.reduction[self.thread_id][index] = self.current_scores_indexes[index].len();
                }
            }
        }

        self.sender.send(Response::from(is_covering)).unwrap();
        *row_id += 1;

        (false, is_covering)
    }

    pub(crate) fn run(&mut self) {
        loop {
            match self.receiver.recv() {
                #[cfg(feature = "threaded-fill")]
                Ok(Work::FillMCA) => new_mca::<ValueId, ParameterId, S_STRENGTH, STRENGTH>(self.thread_id, &self.ipog_data.parameters, self.thread_count + 1, &mut self.ipog_data.mca),
                Ok(Work::NextParameter) => {
                    while Ok(true) != self.constraint_receiver.recv() {};

                    let at_parameter = self.ipog_data.at_parameter_main.load(SeqCst);
                    let pc_list_len = self.ipog_data.pc_list.sizes[at_parameter - STRENGTH];
                    let value_choices: ValueId = self.ipog_data.parameters[at_parameter];

                    let mut row_id = 1;
                    let (start, end) = split(self.thread_count, self.thread_id, pc_list_len);

                    unsafe { self.ipog_data.cm.set_zero_covered_sub(start, end); }

                    let mut done = false;
                    let mut expect_result = false;
                    while !done {
                        let temp = self.thread_high_score(expect_result, &mut row_id, value_choices, start, end);
                        done = temp.0;
                        expect_result = temp.1;
                        if expect_result {
                            std::mem::swap(&mut self.current_scores_indexes, &mut self.previous_scores_indexes);
                        }
                    }
                    self.sender.send(Response::Done).unwrap();
                }
                Ok(Work::SetCovered(_)) => { panic!("Did not expect cover message!"); }
                Ok(_) => {}
                Err(_) => { return; }
            }
        }
    }
}

pub(crate) fn init_thread_pool<ValueId: Id, ParameterId: Id,  const STRENGTH: usize>(sut: Arc<ConstrainedSUT<ValueId, ParameterId>>, ipog_data_arc: Arc<Wrapper<ValueId, ParameterId, S_STRENGTH, STRENGTH>>) -> (Vec<Sender<Work<ValueId>>>, Vec<Receiver<Response>>, Receiver<()>) {
    let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
    let thread_count: usize = ipog_data.thread_count / 2;
    let mut senders = Vec::with_capacity(thread_count + 1);
    let mut receivers = Vec::with_capacity(thread_count);
    let mut constraint_senders = Vec::with_capacity(thread_count);

    let constraint_data = Arc::new(ConstraintResults {
        results: UnsafeCell::new(Default::default()),
        result_ids: UnsafeCell::new([0; CONSTRAINT_BUFFER_SIZE]),
    });

    for thread_id in 0..thread_count {
        let (sender, local_receiver) = bounded(CHANNEL_BOUNDS);
        senders.push(sender);
        let (local_sender, receiver) = bounded(CHANNEL_BOUNDS);
        receivers.push(receiver);
        let (sender, local_constraint_receiver) = bounded(CONSTRAINT_CHANNEL_SIZE);
        constraint_senders.push(sender);
        let local_ipog_data_arc = ipog_data_arc.clone();
        let local_constraint_data = constraint_data.clone();

        thread::spawn(move || {
            ThreadMain::new(thread_id, local_ipog_data_arc, local_sender, local_receiver, local_constraint_receiver, local_constraint_data, thread_count).run();
        });
    }

    let (sender, local_receiver) = bounded(CHANNEL_BOUNDS);
    senders.push(sender);
    let (main_sender, constraint_receiver) = bounded(CHANNEL_BOUNDS);

    thread::spawn(move || {
        thread_constraints::<ValueId, ParameterId, S_STRENGTH, STRENGTH>(sut, ipog_data_arc, thread_count, constraint_senders, main_sender, local_receiver, constraint_data, thread_count);
    });

    debug_assert_eq!(senders.len(), thread_count + 1);
    debug_assert_eq!(receivers.len(), thread_count);
    (senders, receivers, constraint_receiver)
}
