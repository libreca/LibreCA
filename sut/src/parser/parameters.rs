// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use nom::{named, separated_list1};
use nom::bytes::complete::tag;
use nom::IResult;
use nom::multi::many1;

use crate::TemporaryParameter;

use super::{e2s, read_value};

named!(parse_values<&str, Vec<&str>>, separated_list1!(tag(","), read_value));

fn parse_parameter(text: &str) -> IResult<&str, TemporaryParameter> {
    let (text, parameter) = read_value(text)?;
    let (text, _) = tag(":")(text)?;
    let (text, values) = parse_values(text)?;
    let (text, _) = tag(";")(text)?;
    Ok((text, TemporaryParameter { name: parameter.to_string(), values: values.into_iter().map(|s| s.into()).collect() }))
}

pub(crate) fn parse(text: &str) -> Result<(&str, Vec<TemporaryParameter>), String> {
    many1(parse_parameter)(text).map_err(e2s)
}

#[cfg(test)]
mod parameter_tests {
    use super::*;
    use common::{UVec, u_vec};

    #[test]
    fn test_parse_values() {
        assert_eq!(parse_values("  a , b,c ,d, e"), Ok(("", vec!("a", "b", "c", "d", "e"))));
        assert_eq!(parse_values("  a , b,c d, e"), Ok(("d, e", vec!("a", "b", "c"))));
        assert_eq!(parse_values("  a : b,c ,d, e"), Ok((": b,c ,d, e", vec!("a"))));
        assert_eq!(parse_values("  a ; b,c ,d, e"), Ok(("; b,c ,d, e", vec!("a"))));
        assert_eq!(parse_values("  a ,; b,c ,d, e"), Ok((",; b,c ,d, e", vec!("a"))));
        assert_eq!(parse_values("  a ,: b,c ,d, e"), Ok((",: b,c ,d, e", vec!("a"))));
        assert_eq!(parse_values("a"), Ok(("", vec!("a"))));
        assert!(parse_values("   ,: b, d, e").is_err());
        assert!(parse_values("").is_err());
    }

    #[test]
    fn test_parse_parameter_line() {
        assert_eq!(parse_parameter("0:  a , b,c ,d, e;"), Ok(("", TemporaryParameter { name: "0".to_string(), values: u_vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()] })));
        assert_eq!(parse_parameter("0:a;"), Ok(("", TemporaryParameter { name: "0".into(), values: u_vec!["a".into()] })));
        assert!(parse_parameter(" 0:  a : b,c ,d, e;").is_err());
        assert!(parse_parameter(" 0:  a : b,c ,d, e").is_err());
        assert!(parse_parameter(" 0 :  a  b,c ,d, e;").is_err());
        assert!(parse_parameter(" 0 :  a  b,c ,d, e").is_err());
        assert!(parse_parameter("0:  a ,; b,c ,d, e;").is_err());
        assert!(parse_parameter("0:  a ,; b,c ,d, e").is_err());
        assert!(parse_parameter("0:  a ,: b,c ,d, e;").is_err());
        assert!(parse_parameter("0:  a ,: b,c ,d, e").is_err());
        assert!(parse_parameter("0 :  a , b,c d, e").is_err());
        assert!(parse_parameter("   ,: b, d, e").is_err());
        assert!(parse_parameter("").is_err());
        assert!(parse_parameter("a").is_err());
    }
}
