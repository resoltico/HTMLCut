// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Allocation-preserving tagged ownership for two Arc payload kinds.
use super::{Arc, ArcBorrow, ArcInner};
use std::{fmt, marker::PhantomData, mem, ptr};

/// A tagged union that can represent `Arc<A>` or `Arc<B>` while only consuming a
/// single word. The type is also `NonNull`, and thus can be stored in an Option
/// without increasing size.
///
/// This is functionally equivalent to
/// `enum ArcUnion<A, B> { First(Arc<A>), Second(Arc<B>)` but only takes up
/// up a single word of stack space.
pub struct ArcUnion<A, B> {
    p: ptr::NonNull<()>,
    phantom_a: PhantomData<A>,
    phantom_b: PhantomData<B>,
}

unsafe impl<A: Sync + Send, B: Send + Sync> Send for ArcUnion<A, B> {}
unsafe impl<A: Sync + Send, B: Send + Sync> Sync for ArcUnion<A, B> {}

impl<A: PartialEq, B: PartialEq> PartialEq for ArcUnion<A, B> {
    fn eq(&self, other: &Self) -> bool {
        use crate::ArcUnionBorrow::*;
        match (self.borrow(), other.borrow()) {
            (First(x), First(y)) => x == y,
            (Second(x), Second(y)) => x == y,
            (_, _) => false,
        }
    }
}

impl<A: Eq, B: Eq> Eq for ArcUnion<A, B> {}

/// This represents a borrow of an `ArcUnion`.
#[derive(Debug)]
pub enum ArcUnionBorrow<'a, A: 'a, B: 'a> {
    First(ArcBorrow<'a, A>),
    Second(ArcBorrow<'a, B>),
}

impl<A, B> ArcUnion<A, B> {
    unsafe fn new(ptr: *mut ()) -> Self {
        ArcUnion {
            p: unsafe { ptr::NonNull::new_unchecked(ptr) },
            phantom_a: PhantomData,
            phantom_b: PhantomData,
        }
    }

    /// Returns true if the two values are pointer-equal.
    #[inline]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        this.p == other.p
    }

    #[inline]
    pub fn ptr(&self) -> ptr::NonNull<()> {
        self.p
    }

    /// Returns an enum representing a borrow of either A or B.
    #[inline]
    pub fn borrow(&self) -> ArcUnionBorrow<'_, A, B> {
        if self.is_first() {
            let ptr = self.p.as_ptr() as *const ArcInner<A>;
            let borrow = unsafe { ArcBorrow::from_inner_ptr(ptr) };
            ArcUnionBorrow::First(borrow)
        } else {
            let ptr = self
                .p
                .as_ptr()
                .map_addr(|address| address & !0x1)
                .cast::<ArcInner<B>>();
            let borrow = unsafe { ArcBorrow::from_inner_ptr(ptr) };
            ArcUnionBorrow::Second(borrow)
        }
    }

    /// Creates an `ArcUnion` from an instance of the first type.
    pub fn from_first(other: Arc<A>) -> Self {
        let union = unsafe { Self::new(other.ptr() as *mut _) };
        mem::forget(other);
        union
    }

    /// Creates an `ArcUnion` from an instance of the second type.
    pub fn from_second(other: Arc<B>) -> Self {
        let union =
            unsafe { Self::new(other.ptr().cast::<()>().map_addr(|address| address | 0x1)) };
        mem::forget(other);
        union
    }

    /// Returns true if this `ArcUnion` contains the first type.
    pub fn is_first(&self) -> bool {
        self.p.as_ptr().addr() & 0x1 == 0
    }

    /// Returns true if this `ArcUnion` contains the second type.
    pub fn is_second(&self) -> bool {
        !self.is_first()
    }

    /// Returns a borrow of the first type if applicable, otherwise `None`.
    pub fn as_first(&self) -> Option<ArcBorrow<'_, A>> {
        match self.borrow() {
            ArcUnionBorrow::First(x) => Some(x),
            ArcUnionBorrow::Second(_) => None,
        }
    }

    /// Returns a borrow of the second type if applicable, otherwise None.
    pub fn as_second(&self) -> Option<ArcBorrow<'_, B>> {
        match self.borrow() {
            ArcUnionBorrow::First(_) => None,
            ArcUnionBorrow::Second(x) => Some(x),
        }
    }
}

impl<A, B> Clone for ArcUnion<A, B> {
    fn clone(&self) -> Self {
        match self.borrow() {
            ArcUnionBorrow::First(x) => ArcUnion::from_first(x.clone_arc()),
            ArcUnionBorrow::Second(x) => ArcUnion::from_second(x.clone_arc()),
        }
    }
}

impl<A, B> Drop for ArcUnion<A, B> {
    fn drop(&mut self) {
        match self.borrow() {
            ArcUnionBorrow::First(x) => {
                let _ = Arc {
                    p: x.ptr,
                    phantom: PhantomData,
                };
            }
            ArcUnionBorrow::Second(x) => {
                let _ = Arc {
                    p: x.ptr,
                    phantom: PhantomData,
                };
            }
        }
    }
}

impl<A: fmt::Debug, B: fmt::Debug> fmt::Debug for ArcUnion<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.borrow(), f)
    }
}

#[cfg(test)]
#[path = "tests/tagged_union.rs"]
mod tests;
