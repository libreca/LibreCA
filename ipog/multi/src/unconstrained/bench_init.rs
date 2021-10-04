// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use test::Bencher;

use common::{u_vec, UVec};
use mca::MCA;

use crate::unconstrained::threads::init_thread_pool;
use crate::Wrapper;

pub(crate) const THREAD_COUNT: usize = 4;
pub(crate) const PARAMETERS: [u8; 15] = [5, 5, 4, 4, 4, 4, 4, 4, 4, 3, 3, 2, 2, 2, 2];

fn mca_size() -> usize {
    let mut length = 1;
    for &p in PARAMETERS.iter().take(6) {
        length *= p as usize;
    }
    length
}


#[bench]
fn bench_initial_single(bencher: &mut Bencher) {
    let length = mca_size();
    let parameters = UVec::from(PARAMETERS.to_vec());

    bencher.iter(|| {
        assert_eq!(mca::new_unconstrained::<u8, u8, 6>(&parameters).array.len(), length);
    })
}


#[cfg(feature = "threaded-fill")]
#[bench]
fn bench_initial_thread(bencher: &mut Bencher) {
    let length = mca_size();
    let mut ipog_data = crate::IPOGData::<u8, u8, 5, 6>::new(UVec::from(PARAMETERS.to_vec()), 0);
    ipog_data.thread_count = THREAD_COUNT;
    let (_start, end) = crate::threads_common::split(THREAD_COUNT, 0, length);
    let parameters = UVec::from(PARAMETERS.to_vec());

    bencher.iter(|| {
        let mut mca = MCA::new_empty();
        unsafe {
            crate::covering_array::new_reserved_mca::<u8, 6>(&parameters, &[], &mut mca);
            assert_eq!(mca.array.len(), length);
            crate::unconstrained::threads::new_mca(0, &mut ipog_data);
            mca.array.set_len(end);
        }
    })
}

#[bench]
fn bench_initial_alloc(bencher: &mut Bencher) {
    let length = mca_size();
    let capacity = length * PARAMETERS.len() * 6;

    bencher.iter(|| {
        let _mca = MCA::<u8> { array: UVec::with_capacity(capacity), dont_care_locations: UVec::with_capacity(capacity), vertical_extension_rows: UVec::with_capacity(capacity), new_row: UVec::with_capacity(PARAMETERS.len()) };
    })
}

#[test]
fn uninitialised_mca() {
    let mut mca = MCA::<u8> { array: UVec::with_capacity(100), dont_care_locations: UVec::with_capacity(100), vertical_extension_rows: UVec::with_capacity(0), new_row: UVec::with_capacity(0) };
    unsafe { mca.array.set_len(10); }
    let mut pointer = mca.array.as_mut_ptr();
    let row = u_vec![1, 2, 3, 4, 5];
    for _ in 0..10 {
        unsafe { pointer.write(row.clone()) };
        pointer = unsafe { pointer.offset(1) };
    }

    assert_eq!(mca.array.len(), 10);
    for new_row in mca.array {
        assert_eq!(new_row, row);
    }
}

#[bench]
fn bench_initial_single_fill(bencher: &mut Bencher) {
    let length = mca_size();
    let capacity = length * PARAMETERS.len() * 6;

    bencher.iter(|| {
        let mut array = u_vec![u_vec![37; PARAMETERS.len()]; length];
        array.reserve(capacity);
    })
}


#[bench]
fn bench_initial_1d(bencher: &mut Bencher) {
    let length = mca_size();
    let capacity = length * PARAMETERS.len() * 6;

    bencher.iter(|| {
        let mut array = u_vec![37; PARAMETERS.len() * length];
        array.reserve(PARAMETERS.len() * capacity);
    })
}


#[cfg(feature = "threaded-fill")]
#[bench]
fn bench_initial_thread_init(bencher: &mut Bencher) {
    let length = mca_size();
    bencher.iter(|| {
        let mut mca = MCA::new_empty();
        unsafe { crate::covering_array::new_reserved_mca::<u8, 6>(&PARAMETERS, &[], &mut mca) };
        assert_eq!(mca.array.len(), length);
        unsafe { mca.array.set_len(1) };
    })
}

#[cfg(feature = "threaded-fill")]
#[bench]
fn bench_initial_thread_full(bencher: &mut Bencher) {
    bencher.iter(|| {
        let ipog_data_arc = Wrapper::<u8, u8, 5, 6>::new(UVec::from(PARAMETERS.to_vec()), 0);
        let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
        ipog_data.thread_count = THREAD_COUNT;

        let (senders, receivers) = init_thread_pool(ipog_data_arc.clone());
        unsafe { crate::covering_array::new_reserved_mca::<u8, 6>(&ipog_data.parameters, &senders, &mut ipog_data.mca) };

        for receiver in receivers {
            let _a = receiver.recv().unwrap();
            assert_eq!(receiver.recv().unwrap(), crate::threads_common::Response::Done);
        }
    });
}

#[bench]
fn bench_initial_start_one_thread(bencher: &mut Bencher) {
    bencher.iter(|| {
        let ipog_data_arc = Wrapper::<u8, u8, 6>::new(UVec::from(PARAMETERS.to_vec()), 0);
        let ipog_data = unsafe { &mut *ipog_data_arc.data.get() };
        ipog_data.thread_count = 1;
        let (_senders, _receivers) = init_thread_pool(ipog_data_arc.clone());
    });
}

#[bench]
fn bench_initial_start_threads(bencher: &mut Bencher) {
    bencher.iter(|| {
        let ipog_data_arc = Wrapper::<u8, u8, 6>::new(UVec::from(PARAMETERS.to_vec()), 0);
        let (_senders, _receivers) = init_thread_pool(ipog_data_arc.clone());
    });
}

#[bench]
fn bench_initial_data_init(bencher: &mut Bencher) {
    bencher.iter(|| {
        let _ipog_data_arc = Wrapper::<u8, u8, 6>::new(UVec::from(PARAMETERS.to_vec()), 0);
    });
}
