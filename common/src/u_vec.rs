// Copyright 2021 A Veenstra.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your option. This file may not be copied,
// modified, or distributed except according to those terms.

use std::iter::FromIterator;
use std::ops::Range;

/// This struct wraps a [Vec] and uses unsafe methods to access its contents if `debug_assertions` is off.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct UVec<T>(Vec<T>);

impl<T> UVec<T> {
    /// See [Vec::with_capacity].
    #[inline]
    pub fn with_capacity(size: usize) -> Self { Self(Vec::with_capacity(size)) }
    /// See [Vec::push].
    #[inline]
    pub fn push(&mut self, value: T) { self.0.push(value) }
    /// See [slice::swap].
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) { self.0.swap(a, b); }
    /// See [Vec::remove].
    #[inline]
    pub fn remove(&mut self, index: usize) -> T { self.0.remove(index) }
    /// See [slice::get].
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> { self.0.get(index) }
    /// See [slice::get_mut].
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> { self.0.get_mut(index) }
    /// See [Vec::remove].
    #[inline]
    pub fn pop(&mut self) -> Option<T> { self.0.pop() }
    /// See [Vec::truncate].
    #[inline]
    pub fn truncate(&mut self, len: usize) { self.0.truncate(len) }
    /// See [Vec::reserve].
    #[inline]
    pub fn reserve(&mut self, additional: usize) { self.0.reserve(additional) }
    /// See [Vec::clear].
    #[inline]
    pub fn clear(&mut self) { self.0.clear() }
    /// See [Vec::retain].
    #[inline]
    pub fn retain<F>(&mut self, f: F) where F: FnMut(&T) -> bool { self.0.retain(f) }
    /// See [Vec::capacity].
    #[inline]
    pub fn capacity(&self) -> usize { self.0.capacity() }
    /// See [Vec::len].
    #[inline]
    pub fn len(&self) -> usize { self.0.len() }
    /// See [Vec::is_empty()].
    #[inline]
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    /// See [Vec::set_len()].
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) { self.0.set_len(new_len) }
    /// See [Vec::as_mut_ptr()].
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T { self.0.as_mut_ptr() }
    /// See [slice::iter].
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<T> { self.0.iter() }
    /// See [slice::as_chunks_unchecked].
    #[inline]
    pub unsafe fn as_chunks_unchecked<const N: usize>(&self) -> &[[T; N]] { self.0.as_chunks_unchecked::<N>() }
    /// See [slice::iter_mut].
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> { self.0.iter_mut() }
    /// Unwrap the [UVec] and return a [Vec].
    #[inline]
    pub fn unwrap(self) -> Vec<T> { self.0 }
    /// Unwrap the [UVec] and borrow a [Vec].
    #[inline]
    pub fn unwrap_ref(&self) -> &Vec<T> { &self.0 }
    /// Unwrap the [UVec] and mut borrow a [Vec].
    #[inline]
    pub fn unwrap_ref_mut(&mut self) -> &mut Vec<T> { &mut self.0 }
    /// Return a [slice] of the underlying [Vec].
    #[inline]
    pub fn as_slice(&self) -> &[T] { &self.0 }
    /// Return a mutable [slice] of the underlying [Vec].
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [T] { &mut self.0 }
}

impl<T> std::ops::Index<usize> for UVec<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        if cfg!(debug_assertions) {
            &self.0[index]
        } else {
            unsafe { self.0.get_unchecked(index) }
        }
    }
}

impl<T> std::ops::Index<std::ops::Range<usize>> for UVec<T> {
    type Output = [T];

    #[inline]
    fn index(&self, index: Range<usize>) -> &Self::Output {
        if cfg!(debug_assertions) {
            &self.0[index]
        } else {
            unsafe { self.0.get_unchecked(index) }
        }
    }
}

impl<T> std::ops::Index<std::ops::RangeTo<usize>> for UVec<T> {
    type Output = [T];

    #[inline]
    fn index(&self, index: std::ops::RangeTo<usize>) -> &Self::Output {
        if cfg!(debug_assertions) {
            &self.0[index]
        } else {
            unsafe { self.0.get_unchecked(index) }
        }
    }
}

impl<T> std::ops::IndexMut<usize> for UVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if cfg!(debug_assertions) {
            &mut self.0[index]
        } else {
            unsafe { self.0.get_unchecked_mut(index) }
        }
    }
}

impl<T> Default for UVec<T> {
    fn default() -> Self { Self(Vec::default()) }
}

impl<T> IntoIterator for UVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}

impl<T> FromIterator<T> for UVec<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self { Self(Vec::from_iter(iter)) }
}

impl<T> From<Vec<T>> for UVec<T> {
    fn from(inner: Vec<T>) -> Self { Self(inner) }
}

impl<T: std::fmt::Debug> std::fmt::Debug for UVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for UVec<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        &self.0 == other
    }
}

/// Same as [vec], but returns a UVec instead.
#[macro_export]
macro_rules! u_vec {
    ($elem:expr; $n:expr) => (UVec::from(vec![$elem; $n]));
    ($($x:expr),+ $(,)?) => (UVec::from(vec![$($x),+]));
}
