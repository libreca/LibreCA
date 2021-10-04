// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use super::*;

#[test]
fn test_validate_strength() {
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "2", "ignored", "-c"])
    )
    .is_ok());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "8", "ignored", "-n"])
    )
    .is_ok());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "10", "ignored", "-c"])
    )
    .is_ok());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "12", "ignored", "-n"])
    )
    .is_ok());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "0", "ignored", "-c"])
    )
    .is_err());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "1", "ignored", "-n"])
    )
    .is_err());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "a", "ignored", "-c"])
    )
    .is_err());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", " ", "ignored", "-n"])
    )
    .is_err());
    assert!(validate_args(
        get_app("", "", "").get_matches_from(&["exe", "-s", "13", "ignored", "-n"])
    )
    .is_err());
}
