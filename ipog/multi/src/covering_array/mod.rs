// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

//! This module contains a method to create an MCA that can be filled using multiple threads at once.

use crossbeam::channel::Sender;
use mca::{DONT_CARE_FILLED, MCA};

use crate::threads_common::Work;
use common::{Number, UVec, u_vec};

pub(crate) unsafe fn new_reserved_mca<ValueId: Number, LocationsType: Number, const STRENGTH: usize>(parameters: &UVec<ValueId>, senders: &[Sender<Work<ValueId>>], mca: &mut MCA<ValueId, LocationsType>) {
    let mut length: usize = 1;
    for &parameter in parameters.iter().take(STRENGTH) {
        length *= parameter.as_usize();
    }
    let capacity = length * parameters.len() * STRENGTH;

    mca.array.reserve(capacity);
    mca.array.push(u_vec![ValueId::default(); parameters.len()]);
    mca.array.set_len(length);

    for sender in senders {
        sender.send(Work::FillMCA).unwrap();
    }

    let mut dont_care_locations = u_vec![LocationsType::mask_high(STRENGTH); mca.array.len()];
    dont_care_locations.reserve(length - mca.array.len());
    dont_care_locations[0] = 0;
    mca.dont_care_locations = dont_care_locations;

    mca.new_row = u_vec![ValueId::dont_care(); parameters.len()];
}
