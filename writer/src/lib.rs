// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains the methods for writing the resulting [MCA] to a file.

#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use common::{DONT_CARE_TEXT, Id};
use sut::SUT;
use mca::MCA;

const DONT_CARE_TEXT_BYTES: &[u8] = DONT_CARE_TEXT.as_bytes();

fn write_value<ValueId: Id, ParameterId: Id>(
    file: &mut BufWriter<File>,
    sut: &SUT<ValueId, ParameterId>,
    index: usize,
    value: ValueId,
) -> std::io::Result<()> {
    match sut.values[index].get(value.as_usize()) {
        Some(text) => file.write_all(text.as_ref()),
        None => file.write_all(DONT_CARE_TEXT_BYTES),
    }
}

/// Write the given [MCA] to the given filename.
pub fn write_result<ValueId: Id, ParameterId: Id>(
    sut: &SUT<ValueId, ParameterId>,
    mca: MCA<ValueId>,
    filename: PathBuf,
) -> std::io::Result<()> {
    write_result_iterable(
        sut,
        mca.array.len(),
        mca.array.into_iter().flatten(),
        filename,
    )
}

fn write_headers<ValueId: Id, ParameterId: Id>(
    sut: &SUT<ValueId, ParameterId>,
    mca_size: usize,
    file: &mut BufWriter<File>,
) -> std::io::Result<()> {
    file.write_all(b"#  '*' represents don't care value\n")?;
    file.write_all(format!("# Number of parameters: {}\n", sut.parameters.len()).as_ref())?;
    file.write_all(format!("# Number of configurations: {}\n", mca_size).as_ref())?;
    let mut parameters_iter = sut.parameter_names.iter();
    file.write(parameters_iter.next().expect("No parameters?").as_bytes())?;
    for parameter in parameters_iter {
        file.write_all(b",")?;
        file.write_all(parameter.as_bytes())?;
    }
    file.write_all(b"\n")
}

fn write_values<I, ValueId: Id, ParameterId: Id>(
    sut: &SUT<ValueId, ParameterId>,
    mca_size: usize,
    mut mca: I,
    mut file: &mut BufWriter<File>,
) -> std::io::Result<()>
    where
        I: Iterator<Item=ValueId>,
{
    for _ in 0..mca_size {
        write_value(&mut file, sut, 0, mca.next().unwrap())?;

        for index in 1..sut.parameters.len() {
            file.write_all(b",")?;
            write_value(&mut file, sut, index, mca.next().unwrap())?;
        }
        file.write_all(b"\n")?;
    }
    Ok(())
}

/// Write the provided [Iterator] to a file.
pub fn write_result_iterable<'a, I, ValueId: Id, ParameterId: Id>(
    sut: &SUT<ValueId, ParameterId>,
    mca_size: usize,
    mca: I,
    filename: PathBuf,
) -> std::io::Result<()>
    where
        I: Iterator<Item=ValueId>,
{
    println!("The resulting suite has {} tests", mca_size);
    let mut writer = BufWriter::new(File::create(filename)?);
    write_headers(sut, mca_size, &mut writer)?;
    write_values(sut, mca_size, mca, &mut writer)?;
    writer.flush()
}
