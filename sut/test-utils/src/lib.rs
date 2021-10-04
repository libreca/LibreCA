// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This crate contains a directory walker.
//!
//! It is only used during testing.

use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::PathBuf;

/// Walks through the given directory and yield all `cocoa` files.
pub struct Walker {
    current_path: PathBuf,
    current_dir: std::fs::ReadDir,
    sub_walker: Option<Box<Walker>>,
}

impl Walker {
    pub fn new(path: PathBuf) -> Self {
        let current_path = path.canonicalize().unwrap();
        Walker {
            current_path: current_path.clone(),
            current_dir: current_path.read_dir().expect("Could not read the provided path"),
            sub_walker: None,
        }
    }
}

impl Iterator for Walker {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        for _ in 0..1000 {
            if let Some(walker) = &mut self.sub_walker {
                match walker.next() {
                    None => { self.sub_walker = None; }
                    path => { return path; }
                }
            }

            if let Some(path) = self.current_dir.next() {
                let path = path.unwrap().path();
                if self.current_path == path.parent().unwrap() {
                    if path.is_dir() {
                        self.sub_walker = Some(Box::new(Walker::new(path)));
                    } else if path.extension() == Some(OsStr::new("cocoa")) {
                        return Some(read_to_string(path).unwrap());
                    }
                }
            } else { return None; }
        }
        panic!("Did not find a benchmark after 1000 iterations. Did you run the test in the package root?");
    }
}
