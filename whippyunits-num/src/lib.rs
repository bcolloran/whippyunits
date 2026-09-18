#![no_std]
#![cfg(feature = "nightly-gca")]
#![cfg_attr(feature = "nightly-gca", allow(incomplete_features))]
#![cfg_attr(
    feature = "nightly-gca",
    feature(min_generic_const_args, generic_const_args, generic_const_items)
)]

//! Experimental native exponent arithmetic, pinned to nightly-2026-09-13.
//!
//! Kept separate so experimental const semantics do not affect unit metadata.
//! Publicly reexported as `whippyunits_core::num` with `nightly-gca` enabled.

/// Const-generic signed exponent with native type-level arithmetic.
///
/// Addition, subtraction and negation evaluate checked i16 operations. A bare
/// arithmetic trait bound does not prove the resulting value cannot overflow;
/// forcing an overflowing output is a compile error. Halving instead requires
/// an evenness witness on the implementation itself.
///
/// Named expressions preserve generic output identities. The compiler does
/// not prove algebraic identities such as adding zero for unresolved inputs.
/// Numeric rescaling tables have independent, narrower supported ranges.
pub struct N<const X: i16>;

/// Higher order type of [`N`] numbers.
///
/// This trait is sealed and cannot be implemented.
pub trait Num: num_seal::Sealed {
    /// The value of this number.
    const N: i16;
}

impl<const X: i16> Num for N<X> {
    const N: i16 = X;
}

mod num_seal {
    pub trait Sealed {}

    impl<const X: i16> Sealed for super::N<X> {}
}

mod gca;

// ---------------------------------------------------------------------------
// Type-level equality
// ---------------------------------------------------------------------------
//
// The equality counterpart to the `Add`/`Sub`/`Neg` arithmetic above: comparing
// two `N` numerals at the type level, yielding a [`typenum::Bit`] (`B1` if
// equal, else `B0`). Stable builds use the integer bridge; nightly builds
// compare native constants and adapt the resulting boolean to the same bits.
//
// A `Bit` result — rather than a mere `where A == B` bound — is what lets a
// caller *branch* on equality at the type level (e.g. deciding whether a list
// of units is uniform), which a bound alone cannot express.

/// Type-level equality of two [`N`] numerals: [`Output`](NumEq::Output) is a
/// [`typenum::Bit`], `B1` iff the two numerals are equal.
pub trait NumEq<Rhs> {
    /// The [`typenum::Bit`] `B1` (equal) or `B0` (unequal).
    type Output;
}

// ---------------------------------------------------------------------------
// Even-only halving (numeral square root)
// ---------------------------------------------------------------------------
//
// The multiplicative dual of the `Add`/`Sub` arithmetic: halving a numeral,
// which — unlike them — is a *partial* operation. It is defined exactly when the
// numeral is even, so a `where N<X>: Halve` bound is a compile-time *proof* that
// `X` is even, and `<N<X> as Halve>::Output` is `N<X/2>`. That is precisely what
// a unit square root needs (halve every exponent), which exists iff every
// exponent is even. Stable builds use typenum PartialDiv; nightly builds use
// a boolean evenness predicate with a true-only witness.

/// Type-level halving of an [`N`] numeral, defined only for even numerals.
///
/// [`Output`](Halve::Output) is `N<X/2>` when `X` is even. When `X` is odd there
/// is simply no impl, so a bound `N<X>: Halve` doubles as a compile-time witness
/// that `X` is even — the exact precondition for a unit square root.
///
/// Odd exponents have no halving implementation:
///
/// ```compile_fail
/// use whippyunits_num::{Halve, N};
/// fn halve<T: Halve>() {}
/// halve::<N<3>>();
/// ```
pub trait Halve {
    /// Half of the numeral (`N<X/2>`); only exists when `X` is even.
    type Output;
}

/// AND-fold of a cons-list of [`typenum::Bit`]s, written as nested 2-tuples
/// terminated by `()`. [`Output`](BitAll::Output) is `B1` iff every element is
/// `B1` (the empty list is vacuously `B1`).
///
/// This is the combinator that reduces a sequence of per-component
/// [`NumEq`] results to a single verdict — e.g. "all twelve exponents of two
/// units agree" — without spelling out an eleven-deep `BitAnd` chain by hand.
pub trait BitAll {
    /// The AND of every bit in the list.
    type Output;
}

impl BitAll for () {
    type Output = typenum::B1;
}

impl<H, T> BitAll for (H, T)
where
    T: BitAll,
    H: core::ops::BitAnd<<T as BitAll>::Output>,
{
    type Output = <H as core::ops::BitAnd<<T as BitAll>::Output>>::Output;
}
