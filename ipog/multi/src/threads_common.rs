// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains methods used commonly in the multithreaded implementation.

use std::cmp::min;

use common::{Id, UVec};

/// The channel between the main thread and the workers is bounded. This constant determines the size of the channel.
pub const CHANNEL_BOUNDS: usize = 32;

/// The work is split in smaller parts before being distributed between threads. This constant determines the number of parts each threads is responsible of.
pub const SUB_SPLIT: usize = 8;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum Work<ValueId: Id> {
    NextParameter,
    SetCovered(ValueId),
    Covered,

    #[cfg(feature = "threaded-fill")]
    FillMCA,

    #[cfg(feature = "filter-map")]
    Filter,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Response {
    NothingFound,
    Found,
    Done,
}

impl From<bool> for Response {
    fn from(value: bool) -> Self {
        if value {
            Response::Found
        } else {
            Response::NothingFound
        }
    }
}

/// Split the work concerning `total_size` items over `thread_count` threads. Returns the part for which `thread_id` is responsible.
#[inline]
pub fn split(thread_count: usize, thread_id: usize, total_size: usize) -> (usize, usize) {
    let mut batch_size = total_size / thread_count;
    if total_size % thread_count != 0 {
        batch_size += 1;
    }

    let start = thread_id * batch_size;
    let end = if thread_id < thread_count - 1 {
        min((thread_id + 1) * batch_size, total_size)
    } else {
        total_size
    };

    (start, end)
}

/// Split the work concerning `total_size` items in `thread_count` * `SUB_SPLIT` parts. Returns the parts for which `thread_id` is responsible.
#[inline]
pub fn multiple_split(thread_count: usize, thread_id: usize, total_size: usize) -> [(usize, usize); SUB_SPLIT] {
    let mut result = [(0, 0); SUB_SPLIT];
    let mut batch_size = total_size / (thread_count * (SUB_SPLIT - 1));

    if 0 < batch_size {
        let (last, rest) = result.split_last_mut().unwrap();
        for (index, part) in rest.iter_mut().enumerate() {
            part.0 = (index * thread_count + thread_id) * batch_size;
            part.1 = part.0 + batch_size;
        }

        let offset = (SUB_SPLIT - 1) * thread_count * batch_size;
        let left_over = total_size - offset;
        batch_size = left_over / thread_count;
        if left_over % thread_count != 0 {
            batch_size += 1;
        }

        last.0 = min(total_size, offset + batch_size * thread_id);
        last.1 = min(total_size, last.0 + batch_size);
    } else {
        for (index, part) in result.iter_mut().enumerate() {
            part.0 = min(total_size, index * thread_count + thread_id);
            part.1 = min(total_size, part.0 + 1);
        }
    }

    result
}

/// Split the work concerning `total_size` items in `thread_count` parts. Returns an iterator returning the parts for which `thread_id` is responsible.
/// If the `no-cycle-split` feature is set, then the parts are not cycled amongst all threads.
pub fn cycling_split(thread_count: usize, thread_id: usize, total_size: usize) -> impl Iterator<Item=(usize, usize)> {
    debug_assert!(thread_count >= 2);
    let mut result = UVec::with_capacity(thread_count);
    let mut previous = 0;
    let mut left = total_size;

    for thread_count in (2..thread_count + 1).rev() {
        let batch_size = left / thread_count;
        left -= batch_size;
        let next = previous + batch_size;
        result.push((previous, next));
        previous = next;
    }

    result.push((previous, total_size));

    if cfg!(feature="no-cycle-split") {
        vec![result.into_iter().cycle().skip(thread_id).next().unwrap()].into_iter().cycle().skip(0)
    } else {
        result.into_iter().cycle().skip(thread_id)
    }
}

/// Set all indices that no longer count towards the score to zero and return the new score.
#[inline]
pub fn filter_scores(new_vec: &mut UVec<u64>, old_vec: &UVec<u64>) -> usize {
    let mut new_iter = new_vec.iter_mut();
    let mut old_iter = old_vec.iter();
    let mut result = 0;
    while let (Some(mut new), Some(mut old)) = (new_iter.next(), old_iter.next()) {
        while *new != *old {
            while *new < *old {
                if let Some(temp) = new_iter.next() {
                    new = temp;
                } else { return result; }
            }

            while *new > *old {
                if let Some(temp) = old_iter.next() {
                    old = temp;
                } else { return result; }
            }
        }

        *new = 0;
        result += 1;
    }

    result
}
