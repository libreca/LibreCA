// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::sync::{Arc, Barrier, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{JoinHandle, spawn};
use test::Bencher;

use crossbeam::utils::Backoff;

use crate::threads_common::split;
use crate::unconstrained::bench_horizontal::{BASIC_MCA, TestData};
use crate::unconstrained::bench_init::{PARAMETERS, THREAD_COUNT};

pub const REPEATS: usize = 20;


struct TestThreads {
    senders: Vec<crossbeam::channel::Sender<bool>>,
    threads: Vec<JoinHandle<()>>,
}

fn run_comm_bench(bencher: &mut Bencher, main_fn: impl Fn(usize) -> (), thread_fns: Vec<impl Fn(usize) -> () + Send + 'static>) {
    let threads = TestThreads::new(thread_fns);

    bencher.iter(|| {
        threads.start();
        for index in 0..REPEATS {
            main_fn(index);
        }
    });
}

impl TestThreads {
    fn new(thread_fns: Vec<impl Fn(usize) -> () + Send + 'static>) -> Self {
        assert_eq!(thread_fns.len(), THREAD_COUNT);
        let test_data = TestData::new(&PARAMETERS);

        let mut result = Self {
            senders: Vec::with_capacity(THREAD_COUNT),
            threads: Vec::with_capacity(THREAD_COUNT),
        };

        for (thread_id, thread_fn) in thread_fns.into_iter().enumerate() {
            let mut test_data_local = test_data.clone();
            let (start, end) = split(THREAD_COUNT, thread_id, test_data_local.pc_list_len / 4);
            let (sender, receiver) = crossbeam::channel::bounded(5);
            result.senders.push(sender);
            result.threads.push(spawn(move || {
                assert!(receiver.recv().unwrap());
                while receiver.recv() == Ok(true) {
                    for index in 0..REPEATS {
                        waste_time(&mut test_data_local, start, end, index + thread_id);
                        thread_fn(index);
                    }
                }
            }));
        }

        result.start();

        result
    }

    fn start(&self) {
        for sender in self.senders.iter() {
            sender.send(true).unwrap();
        }
    }
}

impl Drop for TestThreads {
    fn drop(&mut self) {
        for sender in self.senders.iter() {
            sender.send(false).unwrap();
        }
        for thread in self.threads.drain(..) {
            thread.join().unwrap();
        }
    }
}

fn waste_time(test_data: &mut TestData, start: usize, end: usize, mut index: usize) {
    index = index % BASIC_MCA.len();
    for score in test_data.scores.iter_mut() {
        score.clear();
    }

    test_data.coverage_map.calculate_scores_sub(&test_data.pc_list, test_data.mca.array[index].as_slice(), test_data.mca.dont_care_locations[index], test_data.no_dont_cares, &mut test_data.scores, start, end);
}

#[bench]
fn no_comm(bencher: &mut Bencher) {
    run_comm_bench(bencher, |_| {}, vec![|_| {}; THREAD_COUNT]);
}

#[bench]
fn barrier(bencher: &mut Bencher) {
    let barrier = Arc::new(Barrier::new(THREAD_COUNT + 1));
    let main_barrier = barrier.clone();
    run_comm_bench(bencher, move |_| { main_barrier.wait(); }, (0..THREAD_COUNT).map(|_| {
        let local_barrier = barrier.clone();
        move |_| { local_barrier.wait(); }
    }).collect());
}

#[bench]
fn backoff_seq_cst(bencher: &mut Bencher) {
    backoff(bencher, Ordering::SeqCst, Ordering::SeqCst);
}

#[bench]
fn backoff_relaxed(bencher: &mut Bencher) {
    backoff(bencher, Ordering::Relaxed, Ordering::Relaxed);
}

#[bench]
fn backoff_acq_rel(bencher: &mut Bencher) {
    backoff(bencher, Ordering::Acquire, Ordering::Release);
}

#[bench]
fn backoff_relaxed_release(bencher: &mut Bencher) {
    backoff(bencher, Ordering::Relaxed, Ordering::Release);
}

#[bench]
fn backoff_relaxed_acquire(bencher: &mut Bencher) {
    backoff(bencher, Ordering::Acquire, Ordering::Relaxed);
}

fn backoff(bencher: &mut Bencher, load_ordering: Ordering, store_ordering: Ordering) {
    let main_counter = Arc::new(AtomicUsize::new(REPEATS));
    let mut sub_counters = Vec::with_capacity(THREAD_COUNT);
    for _ in 0..THREAD_COUNT {
        sub_counters.push(Arc::new(AtomicUsize::new(REPEATS)));
    }
    let main_backoff = Backoff::new();

    let thread_fns = sub_counters.iter().map(|local_counter| {
        let local_counter = local_counter.clone();
        let main_counter = main_counter.clone();
        let local_backoff = Backoff::new();

        move |index| {
            local_backoff.reset();
            while index + 1 != main_counter.load(load_ordering) {
                local_backoff.snooze();
            }
            local_counter.store(index + 1, store_ordering);
        }
    }).collect();

    run_comm_bench(bencher, move |index| {
        main_counter.store(index + 1, store_ordering);
        for sub_counter in sub_counters.iter() {
            main_backoff.reset();
            while index + 1 != sub_counter.load(load_ordering) {
                main_backoff.snooze();
            }
        }
    }, thread_fns);
}

#[bench]
fn channel_verse(bencher: &mut Bencher) {
    let mut receivers = Vec::with_capacity(THREAD_COUNT);
    let mut thread_fns = Vec::with_capacity(THREAD_COUNT);

    for _ in 0..THREAD_COUNT {
        let (sender, receiver) = mpsc::channel();
        receivers.push(receiver);
        thread_fns.push(move |_| { sender.send(()).unwrap(); });
    }

    run_comm_bench(bencher, move |_| {
        for receiver in receivers.iter() {
            receiver.recv().unwrap();
        }
    }, thread_fns);
}

#[bench]
fn channel_inverse(bencher: &mut Bencher) {
    let mut senders = Vec::with_capacity(THREAD_COUNT);
    let mut thread_fns = Vec::with_capacity(THREAD_COUNT);

    for _ in 0..THREAD_COUNT {
        let (sender, receiver) = mpsc::channel();
        senders.push(sender);
        thread_fns.push(move |_| { receiver.recv().unwrap(); });
    }

    run_comm_bench(bencher, move |_| {
        for sender in senders.iter() {
            sender.send(()).unwrap();
        }
    }, thread_fns);
}

#[bench]
fn crossbeam_bounded_verse(bencher: &mut Bencher) {
    crossbeam_verse(bencher, || { crossbeam::channel::bounded(10) });
}

#[bench]
fn crossbeam_bounded_inverse(bencher: &mut Bencher) {
    crossbeam_inverse(bencher, || { crossbeam::channel::bounded(10) });
}

#[bench]
fn crossbeam_unbounded_verse(bencher: &mut Bencher) {
    crossbeam_verse(bencher, crossbeam::channel::unbounded);
}

#[bench]
fn crossbeam_unbounded_inverse(bencher: &mut Bencher) {
    crossbeam_inverse(bencher, crossbeam::channel::unbounded);
}

fn crossbeam_verse(bencher: &mut Bencher, channel: impl Fn() -> (crossbeam::channel::Sender<()>, crossbeam::channel::Receiver<()>)) {
    let mut receivers = Vec::with_capacity(THREAD_COUNT);
    let mut thread_fns = Vec::with_capacity(THREAD_COUNT);

    for _ in 0..THREAD_COUNT {
        let (sender, receiver) = channel();
        receivers.push(receiver);
        thread_fns.push(move |_| { sender.send(()).unwrap(); });
    }

    run_comm_bench(bencher, move |_| {
        for receiver in receivers.iter() {
            receiver.recv().unwrap();
        }
    }, thread_fns);
}

fn crossbeam_inverse(bencher: &mut Bencher, channel: impl Fn() -> (crossbeam::channel::Sender<()>, crossbeam::channel::Receiver<()>)) {
    let mut senders = Vec::with_capacity(THREAD_COUNT);
    let mut thread_fns = Vec::with_capacity(THREAD_COUNT);

    for _ in 0..THREAD_COUNT {
        let (sender, receiver) = channel();
        senders.push(sender);
        thread_fns.push(move |_| { receiver.recv().unwrap(); });
    }

    run_comm_bench(bencher, move |_| {
        for sender in senders.iter() {
            sender.send(()).unwrap();
        }
    }, thread_fns);
}
