// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use nom::{delimited, exact, IResult, named, tag};
use nom::branch::alt;
use nom::bytes::complete::{is_a, tag};
use nom::combinator::opt;
use nom::error::{ErrorKind, Error};
use nom::multi::many0;
use nom::sequence::preceded;

use crate::expr;

use super::{e2s, read_value};

named!(eof<&str, Option<&str>>, exact!(opt(is_a(" \t\r\n"))));

fn not(text: &str) -> IResult<&str, Box<dyn expr::Expr>> {
    let (text, sub) = preceded(tag("!"), parse_expr)(text)?;
    Ok((text, Box::new(expr::Not { sub })))
}

fn eq(text: &str) -> IResult<&str, Box<dyn expr::Expr>> {
    let (text, parameter) = read_value(text)?;
    let (text, _) = tag("=")(text)?;
    let (text, value) = read_value(text)?;
    Ok((text, Box::new(expr::Eq { parameter: parameter.to_string(), value: value.to_string() })))
}

named!(par<&str, Box<dyn expr::Expr>>, delimited!(tag!("("), parse_expr, tag!(")")));

fn bin_op_sub<'a>(op_tag: &'a str, op: expr::BOp) -> impl Fn(&'a str) -> IResult<&'a str, expr::BOp> {
    move |text: &'a str| {
        let (rest, _) = tag(op_tag)(text)?;
        Ok((rest, op))
    }
}

fn parse_expr(text: &str) -> IResult<&str, Box<dyn expr::Expr>> {
    let text = text.trim();
    let (text, left) = alt((
        not,
        par,
        eq,
    ))(text)?;
    let text = text.trim_start();

    match alt((
        bin_op_sub("&&", expr::BOp::And),
        bin_op_sub("||", expr::BOp::Or),
        bin_op_sub("=>", expr::BOp::Implies),
    ))(text) {
        Ok((rest, op)) => match parse_expr(rest) {
            Ok((text, right)) => Ok((text, Box::new(expr::BinOp {
                left,
                op,
                right,
            }))),
            Err(_) => Ok((text, left)),
        },
        Err(nom::Err::Error(Error { code: ErrorKind::Tag, .. })) => Ok((text, left)),
        Err(e) => Err(e),
    }
}

fn parse_constraint(text: &str) -> IResult<&str, Box<dyn expr::Expr>> {
    let (text, _) = tag("$assert ")(text.trim_start())?;
    let (text, result) = parse_expr(text)?;
    let (text, _) = tag(";")(text)?;
    Ok((text, result))
}

pub(crate) fn parse(text: &str) -> Result<Vec<Box<dyn expr::Expr>>, String> {
    let (text, constraints) = many0(parse_constraint)(text).map_err(e2s)?;
    eof(text).map_err(e2s)?;
    Ok(constraints)
}
