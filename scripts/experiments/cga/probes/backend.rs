//! Standalone feasibility model, not the production numeral backend.
#![no_std]
#![allow(incomplete_features)]
#![feature(
    min_generic_const_args,
    generic_const_args,
    generic_const_items,
    macroless_generic_const_args,
    macroless_const_item_generic_const_args,
)]

use core::ops::{Add, BitAnd, Neg, Sub};

pub struct N<const X: i16>;
pub const ADD<const A: i16, const B: i16>: i16 = A + B;
pub const SUB<const A: i16, const B: i16>: i16 = A - B;
pub const NEG<const A: i16>: i16 = -A;
pub const HALF<const A: i16>: i16 = A / 2;
pub const EVEN<const A: i16>: bool = A % 2 == 0;
pub const EQ<const A: i16, const B: i16>: bool = A == B;

impl<const A: i16, const B: i16> Add<N<B>> for N<A> {
    type Output = N<{ ADD::<A, B> }>;
    fn add(self, _: N<B>) -> Self::Output { N }
}
impl<const A: i16, const B: i16> Sub<N<B>> for N<A> {
    type Output = N<{ SUB::<A, B> }>;
    fn sub(self, _: N<B>) -> Self::Output { N }
}
impl<const A: i16> Neg for N<A> {
    type Output = N<{ NEG::<A> }>;
    fn neg(self) -> Self::Output { N }
}

pub struct Bool<const VALUE: bool>;
pub trait True {}
impl True for Bool<true> {}
pub trait AsBit { type Output; }
impl AsBit for Bool<true> { type Output = typenum::B1; }
impl AsBit for Bool<false> { type Output = typenum::B0; }

pub trait Halve { type Output; }
impl<const A: i16> Halve for N<A>
where Bool<{ EVEN::<A> }>: True {
    type Output = N<{ HALF::<A> }>;
}
pub trait NumEq<Rhs> { type Output; }
impl<const A: i16, const B: i16> NumEq<N<B>> for N<A>
where Bool<{ EQ::<A, B> }>: AsBit {
    type Output = <Bool<{ EQ::<A, B> }> as AsBit>::Output;
}

pub trait BitAll { type Output; }
impl BitAll for () { type Output = typenum::B1; }
impl<H, T> BitAll for (H, T)
where T: BitAll, H: BitAnd<T::Output> {
    type Output = <H as BitAnd<T::Output>>::Output;
}

// Exact shape used by the production UnitMul implementation, reduced to two
// axes so the probe isolates inference rather than repetitive code generation.
pub struct Unit<const DIM: i16, const SCALE: i16>;
pub trait UnitMul<Rhs> { type Output; }
impl<const A: i16, const B: i16, const S: i16, const T: i16,
     const O: i16, const P: i16> UnitMul<Unit<B, T>> for Unit<A, S>
where N<A>: Add<N<B>, Output = N<O>>, N<S>: Add<N<T>, Output = N<P>> {
    type Output = Unit<O, P>;
}
pub trait UnitSqrt { type Output; }
impl<const A: i16, const S: i16, const O: i16, const P: i16> UnitSqrt for Unit<A, S>
where N<A>: Halve<Output = N<O>>, N<S>: Halve<Output = N<P>> {
    type Output = Unit<O, P>;
}
pub trait UnitEq<Rhs> { type Output; }
impl<const A: i16, const B: i16, const S: i16, const T: i16> UnitEq<Unit<B, T>> for Unit<A, S>
where N<A>: NumEq<N<B>>, N<S>: NumEq<N<T>>,
    (<N<A> as NumEq<N<B>>>::Output, (<N<S> as NumEq<N<T>>>::Output, ())): BitAll {
    type Output = <(<N<A> as NumEq<N<B>>>::Output,
        (<N<S> as NumEq<N<T>>>::Output, ())) as BitAll>::Output;
}

pub struct Uniform;
pub struct Mixed;
pub trait Dispatch { type Output; }
impl Dispatch for typenum::B1 { type Output = Uniform; }
impl Dispatch for typenum::B0 { type Output = Mixed; }
pub trait AutoReduce<Rhs> { type Output; }
impl<A, B> AutoReduce<B> for A
where A: UnitEq<B>, <A as UnitEq<B>>::Output: Dispatch {
    type Output = <<A as UnitEq<B>>::Output as Dispatch>::Output;
}

pub fn explicit_add<const A: i16, const B: i16, const O: i16>(a: N<A>, b: N<B>) -> N<O>
where N<A>: Add<N<B>, Output = N<O>> { a + b }

pub fn nested<const A: i16, const B: i16, const C: i16>(a: N<A>, b: N<B>, c: N<C>)
    -> <<N<A> as Add<N<B>>>::Output as Sub<N<C>>>::Output {
    (a + b) - c
}

pub fn witness_root<U: UnitSqrt>() {}
