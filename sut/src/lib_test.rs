// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use common::{u_vec, UVec};

use crate::parse_constrained;

#[test]
fn test_empty() {
    match parse_constrained("") {
        Ok(_) => panic!("No result should be provided."),
        Err(_) => {}
    }
}

#[test]
fn test_empty_line() {
    match parse_constrained(";") {
        Ok(_) => panic!("No result should be provided."),
        Err(_) => {}
    }
}

#[test]
fn test_single_character() {
    match parse_constrained("a") {
        Ok(_) => panic!("No result should be provided."),
        Err(_) => {}
    }
}

#[test]
fn test_single_entry() {
    match parse_constrained("p1: v1;") {
        Ok(obj) => {
            assert_eq!(obj.sub_sut.parameters, u_vec![1]);
            assert_eq!(obj.sub_sut.parameter_names, u_vec!["p1".to_string()]);
            assert_eq!(obj.sub_sut.values, u_vec![u_vec!["v1".to_string()]])
        }
        Err(e) => panic!("Result for a simple line should not fail: {:?}", e),
    }
}

#[test]
fn test_incorrect_values() {
    match parse_constrained("p1: v1 a;") {
        Ok(_) => panic!("No result should be provided."),
        Err(_) => {}
    }
}

#[test]
fn test_normal() {
    match parse_constrained("p1 : v1, 3;  ") {
        Ok(obj) => {
            assert_eq!(obj.sub_sut.parameters, u_vec![2]);
            assert_eq!(obj.sub_sut.parameter_names, u_vec!["p1".to_string()]);
            assert_eq!(obj.sub_sut.values, u_vec![u_vec!["v1".to_string(), "3".to_string()]])
        }
        Err(e) => panic!("Result for a simple line should not fail: {:?}", e),
    }
}

#[cfg(not(feature = "no-sort"))]
#[test]
fn test_normal_multiple_sorted() {
    match parse_constrained("p1 : v1, 3;\n p2 : v2, 4, true;") {
        Ok(obj) => {
            assert_eq!(obj.sub_sut.parameters, u_vec![3, 2]);
            assert_eq!(obj.sub_sut.parameter_names, u_vec!["p2".to_string(), "p1".to_string()]);
            assert_eq!(obj.sub_sut.values, u_vec![u_vec![
                "v2".to_string(), "4".to_string(), "true".to_string(),
            ], u_vec![
                "v1".to_string(), "3".to_string(),
            ]]);
        }
        Err(e) => panic!("Result for a simple line should not fail: {:?}", e),
    }
}

#[cfg(feature = "no-sort")]
#[test]
fn test_normal_multiple_unsorted() {
    match parse_constrained("p1 : v1, 3;\n p2 : v2, 4, true;") {
        Ok(obj) => {
            assert_eq!(obj.sub_sut.parameters, u_vec![2, 3]);
            assert_eq!(obj.sub_sut.parameter_names, u_vec!["p1".to_string(), "p2".to_string()]);
            assert_eq!(obj.sub_sut.values, u_vec![u_vec![
                "v1".to_string(), "3".to_string(),
            ], u_vec![
                "v2".to_string(), "4".to_string(), "true".to_string(),
            ]]);
        }
        Err(e) => panic!("Result for a simple line should not fail: {:?}", e),
    }
}
