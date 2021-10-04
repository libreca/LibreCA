// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::collections::HashSet;

use sut::parse_unconstrained;

use mca::new_unconstrained;

#[test]
fn test_coverage_map() {
    let sut = match parse_unconstrained(
        "p0: v0, v1;p1: v0, v1, v2;p2: v0, v1, v2;p3: v0, v1;p4: v0, v1;",
    ) {
        Ok(res) => res,
        Err(e) => panic!("Parsing went wrong? {:?}", e),
    };

    let mut mca = new_unconstrained::<usize, usize, 3>(&sut.parameters);

    assert_eq!(mca.array.len(), 2 * 3 * 3);

    let mut set = HashSet::new();

    while mca.array.len() > 1 {
        let test = mca.array.pop().unwrap();
        assert_eq!(test.len(), 5);
        assert!(test[0] < 3);
        assert!(test[1] < 3);
        assert!(test[2] < 2);

        for cell in test.iter().skip(3) {
            assert_eq!(*cell, !0);
        }
        assert!(set.insert(test));

        assert_eq!(mca.dont_care_locations.pop().unwrap(), (!0) << 3);
    }

    let test = mca.array.pop().unwrap();
    for value in test {
        assert_eq!(value, 0);
    }
    assert_eq!(mca.dont_care_locations.pop().unwrap(), 0);
}

#[test]
fn test_big() {
    let sut = match parse_unconstrained(
        "\
    p0: v1, v2, v3, v4, v5, v6, v7;\
    p1: v1, v2, v3, v4, v5, v6;\
    p2: v1, v2, v3, v4, v5;\
    p3: v1, v2, v3, v4, v5;\
    p4: v1, v2;\
    ",
    ) {
        Ok(res) => res,
        Err(e) => panic!("Parsing went wrong? {:?}", e),
    };

    let mca = new_unconstrained::<usize, usize, 4>(&sut.parameters);

    assert_eq!(mca.array.len(), 7 * 6 * 5 * 5);
}
