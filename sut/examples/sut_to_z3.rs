// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

fn main() -> Result<(), String> {
    let mut sut = sut::parse_constrained_file()?;
    let config = z3::Config::new();
    let context = z3::Context::new(&config);
    let solver = sut.get_solver::<sut::Z3Solver>(&context);
    println!("{}", solver.to_string());
    Ok(())
}
