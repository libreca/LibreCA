// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use nom::bytes::complete::{is_a, take_while1};
use nom::combinator::opt;
use nom::IResult;
use std::fmt::Debug;

pub(crate) mod parameters;
pub(crate) mod constraints;

fn e2s<T: Debug>(e: T) -> String {
    format!("{:?}", e)
}

fn is_value_char(input: char) -> bool {
    match input {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true,
        _ => false,
    }
}

fn read_value(input: &str) -> IResult<&str, &str> {
    let (input, _) = opt(is_a(" \t\r\n"))(input)?;
    let (input, result) = take_while1(is_value_char)(input)?;
    let (input, _) = opt(is_a(" \t\r\n"))(input)?;
    Ok((input, result))
}

#[cfg(test)]
mod parser_tests {
    use super::read_value;

    #[test]
    fn test_value_parse() {
        assert_eq!(read_value("a"), Ok(("", "a")));
        assert_eq!(read_value("-a"), Ok(("", "-a")));
        assert_eq!(read_value("test_this"), Ok(("", "test_this")));
        assert_eq!(read_value(" a b "), Ok(("b ", "a")));
        assert!(read_value(" ").is_err());
        assert!(read_value("").is_err());
    }
}

#[cfg(test)]
mod bench {
    extern crate test;

    use test::Bencher;
    use test_utils::Walker;

    #[bench]
    fn bench_benchmarks(b: &mut Bencher) {
        for contents in Walker::new("./".into()) {
            b.iter(|| {
                let (rest, parameters) = super::parameters::parse(&contents).unwrap();
                let _constraints = super::constraints::parse(rest).unwrap();
                assert_ne!(parameters.len(), 0);
            })
        }
    }
}
