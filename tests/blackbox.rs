// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::fs::read_to_string;
use std::process::Command;

use itertools::Itertools;

use common::{u_vec, UVec};
use sut::Solver;

const CARGO: &str = "cargo";
const RELEASE: &str = "--release";
const TCAS_PATH: &str = "sut/tests/benchmarks/tcas";
const TCAS_REFORMAT: &str = "p10: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9;\np11: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9;\np9: 0, 1, 2, 3;\np7: 0, 1, 2;\np8: 0, 1, 2;\np0: 1, 0;\np1: 0, 1;\np2: 0, 1;\np3: 1, 0;\np4: 0, 1;\np5: 0, 1;\np6: 1, 0;\n\n$assert !(p3=0 && p6=1);\n$assert !(p7=0 && p0=0);\n$assert !(p6=0 && p0=1);\n\n\n";
const TCAS_Z3: &str = "(declare-datatypes ((Ex3 0)) (((Ex3_v0) (Ex3_v1) (Ex3_v2))))\n(declare-datatypes ((Ex2 0)) (((Ex2_v0) (Ex2_v1))))\n(declare-fun p6 () Ex2)\n(declare-fun p3 () Ex2)\n(declare-fun p0 () Ex2)\n(declare-fun p7 () Ex3)\n(assert (not (and ((_ is (Ex2_v0 () Ex2)) p3) ((_ is (Ex2_v1 () Ex2)) p6))))\n(assert (not (and ((_ is (Ex3_v1 () Ex3)) p7) ((_ is (Ex2_v0 () Ex2)) p0))))\n(assert (not (and ((_ is (Ex2_v0 () Ex2)) p6) ((_ is (Ex2_v1 () Ex2)) p0))))\n\n";

const LIBRE_CA_SUT: &str = "\
ipog-type: libreca-s, libreca-m;
feature-constraints: constraints-minisat, constraints-z3, constraints-glucose, off;
feature-score: off, score-double, score-single;
feature-filter-map: off, filter-map;
feature-sub-time: off, sub-time;
feature-no-sort: off, no-sort;
feature-cycle: off, no-cycle-split;
flag-constraints: --constraints, --no-constraints;
output-type: ocfs, ocf-, oc-s, oc--, o-fs, o-f-, o--s, o---;

$assert (ipog-type = libreca-s) => (feature-cycle = off);
$assert (ipog-type = libreca-m) => (feature-filter-map = off);
$assert (flag-constraints = --constraints) => (! feature-constraints = off);

$assert (output-type = ocfs) => ((  flag-constraints = --constraints) && (! feature-filter-map = off) && (! feature-no-sort = no-sort));
$assert (output-type = ocf-) => ((  flag-constraints = --constraints) && (! feature-filter-map = off) && (  feature-no-sort = no-sort));
$assert (output-type = oc-s) => ((  flag-constraints = --constraints) && (  feature-filter-map = off) && (! feature-no-sort = no-sort));
$assert (output-type = oc--) => ((  flag-constraints = --constraints) && (  feature-filter-map = off) && (  feature-no-sort = no-sort));
$assert (output-type = o-fs) => ((! flag-constraints = --constraints) && (! feature-filter-map = off) && (! feature-no-sort = no-sort));
$assert (output-type = o-f-) => ((! flag-constraints = --constraints) && (! feature-filter-map = off) && (  feature-no-sort = no-sort));
$assert (output-type = o--s) => ((! flag-constraints = --constraints) && (  feature-filter-map = off) && (! feature-no-sort = no-sort));
$assert (output-type = o---) => ((! flag-constraints = --constraints) && (  feature-filter-map = off) && (  feature-no-sort = no-sort));
";

mod sut_parsing {
    use crate::*;

    fn run_example(example: &str, features: &str, target: &str, expected: &str) {
        let output = Command::new(CARGO)
            .arg("run").arg(format!("--target-dir=target/test-sut-{}", target))
            .arg("--package=sut").arg("--example").arg(example)
            .arg("--features").arg(features)
            .arg("--").arg(TCAS_PATH).output();
        assert!(output.is_ok());
        let output = output.expect("Output is ok but not ok?");
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(String::from_utf8(output.stdout.clone()).unwrap(), expected.to_string(), "{:?}", output);
    }

    #[test]
    fn constraint_count() {
        run_example("constraint_count", "", "sorted", "Constraint count: 3\n");
    }

    #[test]
    fn parameter_count() {
        run_example("parameter_count", "", "sorted", "Parameter count: 12\n");
    }

    #[test]
    fn parameter_levels_sort() {
        run_example("parameter_levels", "", "sorted", "Parameter levels: [10, 10, 4, 3, 3, 2, 2, 2, 2, 2, 2, 2]\n");
    }

    #[test]
    fn parameter_levels_no_sort() {
        run_example("parameter_levels", "no-sort,constraints-z3", "z3", "Parameter levels: [2, 2, 2, 2, 2, 2, 2, 3, 3, 4, 10, 10]\n");
    }

    #[test]
    fn reformat() {
        run_example("reformat", "constraints-minisat", "minisat", TCAS_REFORMAT);
    }

    #[test]
    fn sut_to_z3() {
        run_example("sut_to_z3", "no-sort,constraints-z3", "z3", TCAS_Z3);
    }
}

#[test]
fn get_version() {
    let output = Command::new(CARGO)
        .arg("run").arg("--target-dir=target/test-cli")
        .arg("--bin=libreca-s").arg(RELEASE).arg("--").arg("--version").output();
    assert!(output.is_ok());
    let output = output.expect("Output is ok but not ok?");
    assert!(output.status.success(), "{:?}", output);
}

fn test_feature(ipog_type: String, features: Vec<String>, flags: Vec<String>) -> Result<String, String> {
    let target_dir = format!("target/debug_{}", features.join("_"));
    let output_file = format!("{}/result{}_{}.txt", target_dir, &ipog_type, flags.join("_"));
    let feature_list: String = features.join(",");
    let output = Command::new(CARGO).args([
        "run", "--target-dir", &target_dir, "--bin", &ipog_type, "--features", &feature_list,
        "--", TCAS_PATH, "--strength", "2", "--output", &output_file,
    ]).args(&flags).output();
    if output.is_err() { return Err(format!("{} {}\n{:?}", ipog_type, feature_list, output)); }
    let output = output.expect("Output is ok but not ok?");
    if !output.status.success() {
        return Err(format!(
            "{} {}\nstatus: {:?}\nstderr:\n{}\n\nstdout:\n{}\n\n",
            ipog_type, feature_list, output.status,
            String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout),
        ));
    }
    let output = Command::new(CARGO).args([
        "run", "--target-dir", "target/debug_constraints-minisat", "--release", "--bin", "check-mca",
        "--features", "constraints-minisat", "--", TCAS_PATH, "--strength", "2", "--output", &output_file,
    ]).args(flags).output();
    if output.is_err() { return Err(format!("{:?}", output)); }
    let output = output.expect("Output is ok but not ok?");
    if !output.status.success() {
        return Err(format!(
            "{} {}\nstatus: {:?}\nstderr:\n{}\n\nstdout:\n{}\n\n",
            ipog_type, feature_list, output.status,
            String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout),
        ));
    }
    Ok(output_file)
}

#[test]
fn test_features() {
    let mut libre_ca_sut = sut::parse_constrained(LIBRE_CA_SUT).unwrap();
    let solver_init = sut::SolverImpl::default_init();
    let mut mca = ipog_single::constrained::ConstrainedIPOG::<
        usize, usize, u16, sut::SolverImpl,
        ipog_single::constrained::HorizontalExtension<usize, usize, u16, 2>,
        ipog_single::constrained::VerticalExtension<usize, usize, u16, 2>,
        2
    >::run(&mut libre_ca_sut, &solver_init);

    let mut uses = UVec::with_capacity(libre_ca_sut.sub_sut.parameters.len());
    for parameter in libre_ca_sut.sub_sut.parameters.iter() {
        uses.push(u_vec![0_i32; *parameter]);
    }

    for test in mca.array.iter_mut() {
        for parameter in 0..libre_ca_sut.sub_sut.parameters.len() {
            if test[parameter] != !0 {
                uses[parameter][test[parameter]] += 1;
            }
        }
    }

    let mut solver = sut::SolverImpl::new(&libre_ca_sut, &solver_init);

    for test in mca.array.iter_mut() {
        for parameter in 0..libre_ca_sut.sub_sut.parameters.len() {
            if test[parameter] == !0 {
                for (value, _) in uses[parameter].iter().enumerate().sorted_unstable_by_key(|(_, &v)| v) {
                    test[parameter] = value;
                    if solver.check_row(test.as_slice()) {
                        break;
                    }
                }
                uses[parameter][test[parameter]] += 1;
            }
        }
    }

    let mut results = Vec::with_capacity(mca.array.len());

    let mut has_errors = false;

    for test in mca.array {
        let mut ipog_type: Option<String> = None;
        let mut features = Vec::with_capacity(libre_ca_sut.sub_sut.parameters.len());
        let mut flags = vec![];
        let mut output_type: Option<usize> = None;

        for (parameter, value_id) in test.iter().enumerate() {
            let parameter_name = &libre_ca_sut.sub_sut.parameter_names[parameter];
            if parameter_name == "ipog-type" {
                ipog_type = Some(libre_ca_sut.sub_sut.values[parameter][*value_id].clone());
            } else if parameter_name.starts_with("feature-") {
                let value = &libre_ca_sut.sub_sut.values[parameter][*value_id];
                if value != "off" {
                    features.push(value.clone());
                }
            } else if parameter_name.starts_with("flag-") {
                let value = &libre_ca_sut.sub_sut.values[parameter][*value_id];
                if value != "off" {
                    flags.push(value.clone());
                }
            } else if parameter_name == "output-type" {
                output_type = Some(*value_id);
            }
        }
        match test_feature(ipog_type.expect("No ipog-type set?"), features, flags) {
            Ok(output_path) => results.push((output_type.unwrap(), output_path)),
            Err(error_message) => {
                println!("Error message: {}", error_message);
                has_errors = true;
            }
        }
    }

    let mut last: Vec<Option<(String, String)>> = vec![None; libre_ca_sut.sub_sut.parameters[libre_ca_sut.parameter_to_id["output-type"]]];
    for (output_type, output_path) in results {
        let contents = read_to_string(&output_path).unwrap();
        if let Some((other_path, other_contents)) = &last[output_type] {
            if &contents != other_contents {
                println!("diff {} {}", other_path, output_path);
                has_errors = true;
            }
        } else {
            last[output_type] = Some((output_path, contents));
        }
    }

    assert!(!has_errors);
}
