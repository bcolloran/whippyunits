//! Native exponent arithmetic, pinned experimentally to nightly-2026-09-13.
//!
//! Each calculation has one definition so unresolved generic uses share its
//! identity. Concrete results normalize to the existing N<const i16> type.

use super::{Halve, N, NumEq};
use core::ops::{Add, Neg, Sub};

pub const ADD<const A: i16, const B: i16>: i16 = A + B;
pub const SUB<const A: i16, const B: i16>: i16 = A - B;
pub const NEG<const A: i16>: i16 = -A;
pub const HALF<const A: i16>: i16 = A / 2;
pub const EVEN<const A: i16>: bool = A % 2 == 0;
pub const EQ<const A: i16, const B: i16>: bool = A == B;

impl<const A: i16, const B: i16> Add<N<B>> for N<A> {
    type Output = N<{ core::direct_const_arg!(ADD::<A, B>) }>;

    fn add(self, _: N<B>) -> Self::Output {
        N
    }
}

impl<const A: i16, const B: i16> Sub<N<B>> for N<A> {
    type Output = N<{ core::direct_const_arg!(SUB::<A, B>) }>;

    fn sub(self, _: N<B>) -> Self::Output {
        N
    }
}

impl<const A: i16> Neg for N<A> {
    type Output = N<{ core::direct_const_arg!(NEG::<A>) }>;

    fn neg(self) -> Self::Output {
        N
    }
}

// These witnesses occur in public associated outputs, so their declarations
// must be public even though this backend module is an implementation detail.
pub struct Bool<const VALUE: bool>;
pub trait True {}
impl True for Bool<true> {}

pub trait AsBit {
    type Output;
}
impl AsBit for Bool<true> {
    type Output = typenum::B1;
}
impl AsBit for Bool<false> {
    type Output = typenum::B0;
}

impl<const A: i16> Halve for N<A>
where
    Bool<{ core::direct_const_arg!(EVEN::<A>) }>: True,
{
    type Output = N<{ core::direct_const_arg!(HALF::<A>) }>;
}

impl<const A: i16, const B: i16> NumEq<N<B>> for N<A>
where
    Bool<{ core::direct_const_arg!(EQ::<A, B>) }>: AsBit,
{
    type Output = <Bool<{ core::direct_const_arg!(EQ::<A, B>) }> as AsBit>::Output;
}
