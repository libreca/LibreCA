// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate provides a basic cli for LibreCA.

#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

use std::fs::read_to_string;
use std::path::PathBuf;

pub use clap::crate_version;
use clap::{App, Arg, ArgMatches};
use common::{MAX_STRENGTH, MIN_STRENGTH};
use sut::{parse_constrained, parse_unconstrained, ConstrainedSUT, SUT};

const INPUT_FILE_ARG: &str = "input_file";
const OUTPUT_FILE_ARG: &str = "output_file";
const STRENGTH_ARG: &str = "strength";
const CONSTRAINTS_ARG: &str = "constraints";
const NO_CONSTRAINTS_ARG: &str = "no-constraints";
const EXAMPLE_PREFIX: &str = "examples/";
const BIN_PREFIX: &str = "src/bin/";
const RUST_EXT: &str = ".rs";

/// This enum is returned by the parsing methods of this crate if the result can be both constrained and unconstrained.
pub enum SUTWrapper {
    /// This item wraps around a [ConstrainedSUT].
    Constrained(ConstrainedSUT<usize, usize>),
    /// This item wraps around an unconstrained [SUT].
    Unconstrained(SUT<usize, usize>),
}

fn get_app<'a, 'b>(app_name: &'a str, short_version: &'a str, long_version: &'a str) -> App<'a, 'b>
where
    'a: 'b,
{
    App::new(app_name)
        .version(short_version)
        .long_version(long_version)
        .arg(
            Arg::with_name(INPUT_FILE_ARG)
                .required(true)
                .help("Set the input file with the definition of the system."),
        )
        .arg(
            Arg::with_name(OUTPUT_FILE_ARG)
                .short("o")
                .long("output")
                .required(false)
                .default_value("result.txt")
                .help("Set the output file."),
        )
        .arg(
            Arg::with_name(STRENGTH_ARG)
                .short("s")
                .long("strength")
                .takes_value(true)
                .required(true)
                .help("Set the strength of the resulting test suite."),
        )
        .arg(
            Arg::with_name(CONSTRAINTS_ARG)
                .short("c")
                .long("constraints")
                .conflicts_with(NO_CONSTRAINTS_ARG)
                .required_unless(NO_CONSTRAINTS_ARG)
                .help("Use the constraints in the provided file."),
        )
        .arg(
            Arg::with_name(NO_CONSTRAINTS_ARG)
                .short("n")
                .long("no-constraints")
                .conflicts_with(CONSTRAINTS_ARG)
                .required_unless(CONSTRAINTS_ARG)
                .help("Do not use the constraints in the provided file."),
        )
}

fn validate_args(matches: ArgMatches) -> Result<(PathBuf, PathBuf, usize, bool), String> {
    let input_path = PathBuf::from(
        matches
            .value_of(INPUT_FILE_ARG)
            .ok_or("The input file should be provided")?,
    );

    let output_path = PathBuf::from(
        matches
            .value_of(OUTPUT_FILE_ARG)
            .ok_or("The output file should be provided")?,
    );

    if input_path == output_path {
        return Err("Input and output should not be the same!".to_string())
    }

    let strength = matches
        .value_of(STRENGTH_ARG)
        .ok_or("The strength argument is required.")?
        .parse::<usize>()
        .map_err(|_| "The strength argument should be a number.".to_string())?;

    if strength < MIN_STRENGTH || MAX_STRENGTH < strength {
        Err(format!(
            "Please provide a strength between {} and {}.",
            MIN_STRENGTH, MAX_STRENGTH
        ))
    } else {
        Ok((input_path, output_path, strength, matches.is_present(CONSTRAINTS_ARG)))
    }
}

fn check_sizes(strength: usize, parameters: usize) -> Result<(), String> {
    if strength > parameters {
        Err("Choose a strength equal to or lower than the number of parameters.".into())
    } else {
        Ok(())
    }
}

fn load_sut(args: (PathBuf, PathBuf, usize, bool)) -> Result<(SUTWrapper, PathBuf, usize), String> {
    let contents = read_to_string(args.0).or_else(|e| Err(e.to_string()))?;
    if args.3 {
        let sut = parse_constrained(contents.as_str())?;
        check_sizes(args.2, sut.sub_sut.parameters.len())?;
        if sut.has_constraints() {
            Ok((SUTWrapper::Constrained(sut), args.1, args.2))
        } else {
            Ok((SUTWrapper::Unconstrained(sut.sub_sut), args.1, args.2))
        }
    } else {
        let sut = parse_unconstrained(contents.as_str())?;
        check_sizes(args.2, sut.parameters.len())?;
        Ok((SUTWrapper::Unconstrained(sut), args.1, args.2))
    }
}

/// Parse the commandline arguments and return the [ConstrainedSUT] or [SUT], and strength for which an MCA should be created at the given output path.
pub fn parse_arguments(mut app_name: &str, version: &str) -> Result<(SUTWrapper, PathBuf, usize), String> {
    if app_name.ends_with(RUST_EXT) {
        app_name = &app_name[..app_name.len() - RUST_EXT.len()];
    }

    if app_name.starts_with(EXAMPLE_PREFIX) {
        app_name = &app_name[EXAMPLE_PREFIX.len()..];
    } else if app_name.starts_with(BIN_PREFIX) {
        app_name = &app_name[BIN_PREFIX.len()..];
    }

    let short_version = format!("v{} ({})", version, env!("GIT_HASH_SHORT"));
    let long_version = format!("v{} ({})", version, env!("GIT_HASH"));

    let matches = get_app(app_name, short_version.as_str(), long_version.as_str()).get_matches();

    load_sut(validate_args(matches)?)
}

#[cfg(test)]
mod test_lib;
