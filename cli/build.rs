// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::env::var;
use std::path::Path;
use std::process::Command;

fn main() {
    for (k, v) in std::env::vars() {
        println!("{}: {}", k, v);
    }

    if let Ok(pwd) = var("PWD") {
        let p_pwd = Path::new(&pwd);
        if p_pwd.exists() {
            let output = Command::new("git")
                .args(&["rev-parse", "--short", "HEAD"])
                .current_dir(p_pwd)
                .output()
                .unwrap();
            let git_hash = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=GIT_HASH_SHORT={}", git_hash.trim());
            let output = Command::new("git")
                .args(&["rev-parse", "HEAD"])
                .current_dir(p_pwd)
                .output()
                .unwrap();
            let git_hash = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
            println!(
                "cargo:rustc-rerun-if-changed={}",
                p_pwd.join(".git").join("HEAD").to_str().unwrap()
            );
            return;
        }
    }

    panic!("Could not find git dir of current project.");
}
