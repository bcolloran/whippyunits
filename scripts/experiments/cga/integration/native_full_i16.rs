//! Large native exponents are arithmetic only; this does not extend rescaling tables.
use core::ops::{Add, Neg, Sub};
use whippyunits_core::num::{Halve, N, NumEq};
fn half<A: Halve<Output = O>, O>() {}
fn eq<A: NumEq<B, Output = O>, B, O>() {}
fn add_bound<A: Add<N<1>>>() {}
fn sub_bound<A: Sub<N<1>>>() {}
fn neg_bound<A: Neg>() {}
fn main() {
    let _: N<30000> = (N::<20000> + N::<10000>) + N::<0>;
    let _: N<-32768> = N::<-32767> - N::<1>;
    let _: N<32767> = -N::<-32767>;
    let _: N<0> = N::<-32768> + N::<32767> + N::<1>;
    let _: N<600> = (N::<200> + N::<200>) + N::<200>;
    half::<N<-32768>, N<-16384>>();
    half::<N<32766>, N<16383>>();
    eq::<N<32767>, N<32767>, typenum::B1>();
    eq::<N<-32768>, N<32767>, typenum::B0>();
    // Native Add/Sub/Neg availability alone is not an overflow proof. The
    // separate negative probes force the associated result and must reject it.
    add_bound::<N<32767>>();
    sub_bound::<N<-32768>>();
    neg_bound::<N<-32768>>();
}
