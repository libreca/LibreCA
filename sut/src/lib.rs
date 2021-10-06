// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate provides all the tools necessary to parse `*.cocoa` files.
//! It also provides wrappers around (SAT) solvers.
//!
//! # Features
//! There are a few available features:
//!   * `constraints-common` Feature to remove any errors related to unimplemented solver wrappers.
//!   * `constraints-glucose` Implies MiniSat, but compiles the Glucose drop-in replacement instead.
//!   * `constraints-minisat` Add support for the MiniSat solver.
//!   * `constraints-z3` Add support for the Z3 solver.
//!   * `constraints` Implies `constraints-minisat`.
//!   * `no-sort` Do not sort the parameters based on descending level. IPOG runs better on a sorted SUT.
//!
//! # System Under Test
//! There are two System Under Test (SUT) structures, namely the [SUT] and [ConstrainedSUT].
//! [SUT] is an unconstrained SUT.
//! The [ConstrainedSUT] has a unconstrained variant as one of its fields.
//!
//! # Solvers
//! There currently are two solvers supported:
//!   * [MiniSatSolver], which is the frontend of both MiniSat and Glucose.
//!     Bindings to this SAT solver are provided by [minisat].
//!   * [Z3Solver], which is the frontend of Z3.
//!     Bindings to this SAT solver are provided by [z3].
//!
//! There also is a Solver called [NotASolver], which is used as a placeholder when no solvers are compiled (see the features).
//!
//! # Example
//! ```
//! use sut::{SolverImpl, Solver};
//!
//! if cfg!(feature = "constraints-common") {
//!     let mut c_sut = sut::parse_constrained("
//!         p1: 0, 1, 2;
//!         p2: 0, 1;
//!         p3: 0, 1;
//!
//!         $assert (p1 = 0) => (p3 = 1);
//!     ").expect("Parsing error occurred");
//!     println!("Number of parameters: {}", c_sut.sub_sut.parameters.len());
//!
//!     let solver_init = SolverImpl::default_init();
//!     let mut solver = c_sut.get_solver::<SolverImpl>(&solver_init);
//!     let row = vec![0_usize; c_sut.sub_sut.parameters.len()];
//!
//!     assert!(solver.check_row(&row));
//!     assert_eq!("0", c_sut.sub_sut.values[0][0]);
//!     assert_eq!("0", c_sut.sub_sut.values[1][0]);
//!     assert_eq!("1", c_sut.sub_sut.values[2][0]);
//! }
//!
//! let mut sut = sut::parse_unconstrained("
//!     p1: 0, 1, 2;
//!     p2: 0, 1;
//!     p3: 0, 1;
//!
//!     $assert (p1 = 0) => (p3 = 1);
//! ").expect("Parsing error occurred");
//! println!("Number of parameters: {}", sut.parameters.len());
//! ```

#![cfg_attr(test, feature(test))]
#![deny(missing_docs, rustdoc::missing_crate_level_docs, future_incompatible)]

use std::collections::HashMap;
use std::convert::TryFrom;
use std::env::args;
use std::fmt::{Debug, Error, Formatter};
use std::fs::read_to_string;
use std::marker::PhantomData;
use std::path::Path;

use common::{Number, UVec};
use constraints::find_problem;
pub use constraints::solver::Solver;
#[cfg(feature = "constraints-minisat")]
pub use constraints::solver_minisat::MiniSatSolver;
pub use constraints::solver_not_implemented::NotASolver;
#[cfg(feature = "constraints-z3")]
pub use constraints::solver_z3::Z3Solver;

#[cfg(all(feature = "constraints-z3", not(feature = "constraints-minisat")))]
/// This type points to the default Solver, which currently is [Z3Solver].
///
/// If no features are activated the [NotASolver] is used.
/// If the `constraints-minisat` feature is set, then the [MiniSatSolver] is used.
/// If the `constraints-z3` feature is set (and not the `constraints-minisat` feature), then the [Z3Solver] is used.
///
pub type SolverImpl<'ctx> = Z3Solver<'ctx>;

#[cfg(feature = "constraints-minisat")]
/// This type points to the default Solver, which currently is [MiniSatSolver].
///
/// If no features are activated the [NotASolver] is used.
/// If the `constraints-minisat` feature is set, then the [MiniSatSolver] is used.
/// If the `constraints-z3` feature is set (and not the `constraints-minisat` feature), then the [Z3Solver] is used.
///
pub type SolverImpl<'ctx> = MiniSatSolver;

#[cfg(not(feature = "constraints-common"))]
/// This type points to the default Solver, which currently is [NotASolver].
///
/// If no features are activated the [NotASolver] is used.
/// If the `constraints-minisat` feature is set, then the [MiniSatSolver] is used.
/// If the `constraints-z3` feature is set (and not the `constraints-minisat` feature), then the [Z3Solver] is used.
///
pub type SolverImpl<'ctx> = NotASolver;

mod constraints;
mod expr;
mod parser;

#[cfg_attr(test, derive(Debug, PartialEq))]
struct TemporaryParameter {
    name: String,
    values: UVec<String>,
}

/// Error indicating overflow returned when the [SUT] can not be converted to the specified types.
///
/// Either the number of parameters is higher than the ParameterId can represent or
/// one of the levels exceeds the number of levels the ValueId can represent.
#[derive(Debug)]
pub enum OverflowError {
    /// The Number for the values is not big enough for this [SUT].
    ValueOverflow,

    /// The Number for the parameters is not big enough for this [SUT].
    ParameterOverflow,
}

/// This struct represents the System Under Test (SUT) for which to generate an MCA.
pub struct SUT<ValueId: Number, ParameterId: Number> {
    /// The parameter levels of the SUT.
    pub parameters: UVec<ValueId>,

    /// The names of the parameters.
    pub parameter_names: UVec<String>,

    /// The names of the values.
    ///
    /// The outer vector is indexed by the parameter ID, and the inner vector is indexed by the value ID.
    /// So `sut.values[parameter_id][value_id]`.
    pub values: UVec<UVec<String>>,
    parameter_id: PhantomData<ParameterId>,
}

impl SUT<usize, usize> {
    fn new(mut parameters: Vec<TemporaryParameter>) -> Self {
        let mut result = SUT {
            parameters: UVec::with_capacity(parameters.len()),
            parameter_names: UVec::with_capacity(parameters.len()),
            values: UVec::with_capacity(parameters.len()),
            parameter_id: PhantomData,
        };
        if cfg!(not(feature = "no-sort")) {
            parameters.sort_by_key(|p| !p.values.len());
        }
        for p in parameters.into_iter() {
            result.parameters.push(p.values.len());
            result.parameter_names.push(p.name);
            result.values.push(p.values);
        }
        result
    }

    /// Check if the parameters fit the given ParameterId type.
    pub fn parameters_fit<ParameterId: Number>(&self) -> Result<(), OverflowError> {
        if self.parameters.len() > ParameterId::dont_care().as_usize() {
            Err(OverflowError::ParameterOverflow)
        } else {
            Ok(())
        }
    }

    /// Check if the parameter levels fit the given ParameterId type.
    pub fn values_fit<ValueId: Number>(&self) -> Result<(), OverflowError> {
        if !self.parameters.iter().all(|&e| e < ValueId::dont_care().as_usize()) {
            Err(OverflowError::ValueOverflow)
        } else {
            Ok(())
        }
    }

    /// Mutate from `<usize, usize>` to specific size. Destructive to self.
    pub fn mutate<ValueId: Number, ParameterId: Number>(self) -> SUT<ValueId, ParameterId> {
        SUT {
            parameters: self.parameters.into_iter().map(ValueId::from_usize).collect(),
            parameter_names: self.parameter_names,
            values: self.values,
            parameter_id: PhantomData,
        }
    }
}

#[allow(rustdoc::missing_doc_code_examples)]
impl<'sut, ValueId: Number, ParameterId: Number> TryFrom<&'sut SUT<usize, usize>> for SUT<ValueId, ParameterId> {
    type Error = OverflowError;

    fn try_from(other: &SUT<usize, usize>) -> Result<Self, Self::Error> {
        other.parameters_fit::<ParameterId>()?;
        other.values_fit::<ValueId>()?;

        Ok(Self {
            parameters: other.parameters.iter().map(|&e| ValueId::from_usize(e)).collect(),
            parameter_names: other.parameter_names.clone(),
            values: other.values.clone(),
            parameter_id: PhantomData,
        })
    }
}


/// Represents a [SUT] with constraints.
///
/// Is used to generate solvers for checking the MCA during construction.
pub struct ConstrainedSUT<ValueId: Number, ParameterId: Number> {
    /// The underlying [SUT].
    ///
    /// Changes to the `sub_sut` will break the [ConstrainedSUT] and any related [Solver].
    pub sub_sut: SUT<ValueId, ParameterId>,
    pub(crate) constraints: Vec<Box<dyn expr::Expr>>,
    /// A [HashMap] that allows for the reverse lookup of the parameter ids.
    pub parameter_to_id: HashMap<String, usize>,
    /// A [HashMap] that allows for the reverse lookup of the value ids.
    pub value_to_id: UVec<HashMap<String, usize>>,
}

impl ConstrainedSUT<usize, usize> {
    /// Create a new ConstrainedSUT using the temporary parameters and constraints.
    fn new(parameters: Vec<TemporaryParameter>, constraints: Vec<Box<dyn expr::Expr>>) -> Self {
        let sub_sut = SUT::new(parameters);
        let parameter_to_id = get_parameter_to_id(&sub_sut.parameter_names);
        let value_to_id = get_value_to_id(&sub_sut.values);
        Self { sub_sut, constraints, parameter_to_id, value_to_id }
    }

    /// Check if the parameters fit the given ParameterId type.
    pub fn parameters_fit<ParameterId: Number>(&self) -> Result<(), OverflowError> {
        self.sub_sut.parameters_fit::<ParameterId>()
    }

    /// Check if the parameter levels fit the given ParameterId type.
    pub fn values_fit<ValueId: Number>(&self) -> Result<(), OverflowError> {
        self.sub_sut.values_fit::<ValueId>()
    }

    // `into` not possible due to conflict between ConstrainedSUT<ValueId, ParameterId> and ConstrainedSUT<usize, usize>.
    /// Mutate from `<usize, usize>` to specific size. Destructive to self.
    pub fn mutate<ValueId: Number, ParameterId: Number>(self) -> ConstrainedSUT<ValueId, ParameterId> {
        ConstrainedSUT {
            sub_sut: SUT::try_from(&self.sub_sut).unwrap(),
            constraints: self.constraints,
            parameter_to_id: self.parameter_to_id,
            value_to_id: self.value_to_id,
        }
    }
}

impl<ValueId: Number, ParameterId: Number> ConstrainedSUT<ValueId, ParameterId> {
    /// Get a solver with the constraints loaded.
    ///
    /// Also sorts the values so that the all zeros row is possible.
    /// So the row `u_vec![0; parameter_len]` will always be possible.
    pub fn get_solver<'i, S: Solver<'i>>(&mut self, args: &'i S::Init) -> S {
        let mut solver = S::new(self, args);

        let mut row = vec![ValueId::default(); self.sub_sut.parameters.len()];
        if !solver.check_row(&row) {
            let mut start = 0;
            let end = row.len();
            loop {
                start = find_problem(&mut solver, &row, start, end);
                if end < start {
                    break;
                }
                start -= 1;
                row[start] += ValueId::from_usize(1);
            }

            debug_assert!(solver.check_row(&row[..end - 1]));
            debug_assert!(solver.check_row(&row));

            for (value, (values, value_to_id)) in row.into_iter().zip(self.sub_sut.values.iter_mut().zip(self.value_to_id.iter_mut())) {
                if value != ValueId::default() {
                    values.swap(0, value.as_usize());
                    unsafe { std::ptr::swap(value_to_id.get_mut(&values[0]).unwrap(), value_to_id.get_mut(&values[value.as_usize()]).unwrap()); }
                }
            }

            solver = Solver::new(self, args);
            debug_assert!(solver.check_row(&vec![ValueId::default(); self.sub_sut.parameters.len()]));
        }

        solver
    }

    /// Wrap a [SUT] with a [ConstrainedSUT] without constraints.
    pub fn wrap_sut(sub_sut: SUT<ValueId, ParameterId>) -> Self {
        let parameter_to_id = get_parameter_to_id(&sub_sut.parameter_names);
        let value_to_id = get_value_to_id(&sub_sut.values);
        Self { sub_sut, constraints: vec![], parameter_to_id, value_to_id }
    }

    /// Returns true if the SUT has constraints, otherwise returns false.
    pub fn has_constraints(&self) -> bool {
        !self.constraints.is_empty()
    }

    /// Returns the number of constraints listed in the SUT.
    pub fn count_constraints(&self) -> usize { self.constraints.len() }
}

impl Debug for ConstrainedSUT<usize, usize> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        for (parameter_name, values) in self.sub_sut.parameter_names.iter().zip(self.sub_sut.values.iter()) {
            f.write_str(parameter_name)?;
            f.write_str(": ")?;
            let mut values_iter = values.iter();
            f.write_str(values_iter.next().unwrap())?;
            for value_name in values_iter {
                f.write_str(", ")?;
                f.write_str(value_name)?;
            }
            f.write_str(";\n")?;
        }
        f.write_str("\n")?;
        for constraint in self.constraints.iter() {
            f.write_str("$assert ")?;
            constraint.fmt_no_parenthesis(f)?;
            f.write_str(";\n")?;
        }
        f.write_str("\n")
    }
}

fn get_parameter_to_id(parameter_names: &UVec<String>) -> HashMap<String, usize> {
    let mut result = HashMap::with_capacity(parameter_names.len());
    for p in parameter_names.iter().enumerate() {
        result.insert(p.1.clone(), p.0);
    }
    result
}

fn get_value_to_id(values: &UVec<UVec<String>>) -> UVec<HashMap<String, usize>> {
    let mut result = UVec::with_capacity(values.len());
    for values in values.iter() {
        let mut sub_map = HashMap::with_capacity(values.len());
        for v in values.iter().enumerate() {
            sub_map.insert(v.1.clone(), v.0);
        }
        result.push(sub_map);
    }
    result
}

fn open_file() -> Result<String, String> {
    let args: Vec<String> = args().collect();
    if args.len() != 2 {
        return Err("Not the correct amount of arguments provided!".into());
    }

    let path = Path::new(args[1].as_str());
    if !path.is_file() {
        return Err("Provided file does not exist".into());
    }

    read_to_string(path).map_err(|e| e.to_string())
}

/// Parse the given `str` and return the unconstrained [SUT].
pub fn parse_unconstrained(text: &str) -> Result<SUT<usize, usize>, String> {
    Ok(SUT::new(parser::parameters::parse(text)?.1))
}

/// Parse a file and return the unconstrained [SUT].
///
/// The path to the file is retrieved from the commandline arguments, so these cannot be used for anything else if you use this method.
pub fn parse_unconstrained_file() -> Result<SUT<usize, usize>, String> {
    parse_unconstrained(open_file()?.as_str())
}

/// Parse the given `str` and return the constrained SUT.
pub fn parse_constrained(text: &str) -> Result<ConstrainedSUT<usize, usize>, String> {
    let (text, parameters) = parser::parameters::parse(text)?;
    Ok(ConstrainedSUT::new(parameters, parser::constraints::parse(text)?))
}

/// Parse a file and return the [ConstrainedSUT].
///
/// The path to the file is retrieved from the commandline arguments, so these cannot be used for anything else if you use this method.
pub fn parse_constrained_file() -> Result<ConstrainedSUT<usize, usize>, String> {
    parse_constrained(open_file()?.as_str())
}

/// Parse a file and return the number of parameters found.
///
/// The path to the file is retrieved from the commandline arguments, so these cannot be used for anything else if you use this method.
pub fn get_parameter_count() -> Result<usize, String> {
    Ok(parser::parameters::parse(open_file()?.as_str())?.1.len())
}

/// Parse a file and return the levels of the parameters in descending order.
///
/// The order is not influenced by the `sorted` feature.
///
/// The path to the file is retrieved from the commandline arguments, so these cannot be used for anything else if you use this method.
pub fn get_parameter_levels() -> Result<Vec<usize>, String> {
    let mut levels: Vec<usize> = parser::parameters::parse(open_file()?.as_str())?.1.into_iter().map(|p| p.values.len()).collect();
    if cfg!(not(feature="no-sort")) {
        levels.sort_unstable_by_key(|v| !v);
    }
    Ok(levels)
}

/// Parse a file and return the number of constraints found.
///
/// The path to the file is retrieved from the commandline arguments, so these cannot be used for anything else if you use this method.
pub fn get_constraint_count() -> Result<usize, String> {
    Ok(parser::constraints::parse(
        parser::parameters::parse(open_file()?.as_str())?.0
    )?.len())
}

#[cfg(test)]
mod lib_test;
