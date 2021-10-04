// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use common::UVec;
use pc_list::calculate_length;

use crate::threads_common::{multiple_split, SUB_SPLIT, cycling_split};

#[test]
fn test_split_6_37_4() { test_split(6, 37, 4) }


#[test]
fn test_split_6_10_4() { test_split(6, 10, 4) }

#[test]
fn test_split_6_9_6() { test_split(6, 9, 6) }


#[test]
fn test_split_6_10_6() { test_split(6, 10, 6) }


#[test]
fn test_split_6_10_7() { test_split(6, 10, 7) }

#[test]
fn test_split_6_8_7() { test_split(6, 8, 7) }

#[test]
fn test_split_6_7_7() { test_split(6, 7, 7) }

#[test]
fn test_split_5_6_7() { test_split(5, 6, 7) }

fn test_split(strength: usize, at_parameter: usize, thread_count: usize) {
    test_multiple_split(strength, at_parameter, thread_count);
    test_cycling_split(strength, at_parameter, thread_count);
}
fn test_multiple_split(strength: usize, at_parameter: usize, thread_count: usize) {
    let length = calculate_length(strength, at_parameter);
    let mut split_results = UVec::with_capacity(thread_count);

    for thread_id in 0..thread_count {
        split_results.push(multiple_split(thread_count, thread_id, length));
    }

    println!("s={} p={} tl={} tc={}", strength, at_parameter, length, thread_count);
    for part in 0..SUB_SPLIT {
        for thread_id in 0..thread_count {
            let &(start, end) = &split_results[thread_id][part];
            print!("\t({}, {}; {})", start, end, end - start);
        }
        println!();
    }

    let mut mid = 0;

    for part in 0..SUB_SPLIT {
        for thread_id in 0..thread_count {
            assert_eq!(split_results[thread_id][part].0, mid, "start != previous end? thread: {} part: {}", thread_id, part);
            mid = split_results[thread_id][part].1;
        }
    }

    assert_eq!(length, mid, "Last element does not span the last element.");


    // batch size
    let part_length = split_results[0][0].1;

    // batch sizes should be smaller or equal to first size
    for part in 0..SUB_SPLIT {
        for thread_id in 0..thread_count {
            let sub_part_length = split_results[thread_id][part].1 - split_results[thread_id][part].0;
            assert!((part == SUB_SPLIT - 1 || part_length <= sub_part_length + 1) && sub_part_length <= part_length + 1, "t={} p={} bs={} lbs={}", thread_id, part, part_length, sub_part_length);
        }
    }


    let last_part = SUB_SPLIT - 1;
    let last_part_length = split_results[0][last_part].1 - split_results[0][last_part].0;
    assert!(part_length + 1 >= last_part_length, "{} {}", part_length, last_part_length);
}

fn test_cycling_split(strength: usize, at_parameter: usize, thread_count: usize) {
    let length = calculate_length(strength, at_parameter);
    let mut split_results = UVec::with_capacity(thread_count);

    for thread_id in 0..thread_count {
        split_results.push(cycling_split(thread_count, thread_id, length));
    }

    println!("s={} p={} tl={} tc={}", strength, at_parameter, length, thread_count);
    for _ in 0..SUB_SPLIT {
        for thread_id in 0..thread_count {
            let (start, end) = split_results[thread_id].next().unwrap();
            print!("\t({}, {}; {})", start, end, end as isize - start as isize);
        }
        println!();
    }

    for _ in 0..SUB_SPLIT {
        let mut temp = vec![];
        for thread_id in 0..thread_count {
            temp.push(split_results[thread_id].next().unwrap());
        }

        let mut mid = temp[0].0;

        for thread_id in 0..thread_count {
            assert!(temp[thread_id].0 == mid || (temp[thread_id].0 == 0 && length == mid), "start != previous end? {} {}", mid, temp[thread_id].0);
            mid = temp[thread_id].1;
        }

        assert!(temp.iter().any(|(start, _)| *start == 0), "No start?");
        assert!(temp.iter().any(|(_, end)| *end == length), "No end?");
    }
}
