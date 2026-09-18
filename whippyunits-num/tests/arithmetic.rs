use core::ops::{Add, Neg, Sub};

use whippyunits_num::*;

const LIMIT: i16 = 32;

#[test]
fn can_add_const_numbers() {
    fn assert<A: Add<B, Output = O>, B, O>() {}

    assert::<N<0>, N<0>, N<0>>();

    assert::<N<0>, N<1>, N<1>>();
    assert::<N<1>, N<0>, N<1>>();

    assert::<N<0>, N<-1>, N<-1>>();
    assert::<N<-1>, N<0>, N<-1>>();

    assert::<N<1>, N<2>, N<3>>();
    assert::<N<2>, N<1>, N<3>>();

    assert::<N<1>, N<-2>, N<-1>>();
    assert::<N<2>, N<-1>, N<1>>();

    // Stable range boundary checks
    assert::<N<LIMIT>, N<LIMIT>, N<{ 2 * LIMIT }>>();
    assert::<N<{ -LIMIT }>, N<{ -LIMIT }>, N<{ -2 * LIMIT }>>();
}

#[test]
fn can_sub_const_numbers() {
    fn assert<A: Sub<B, Output = O>, B, O>() {}

    assert::<N<0>, N<0>, N<0>>();

    assert::<N<0>, N<1>, N<-1>>();
    assert::<N<1>, N<0>, N<1>>();

    assert::<N<0>, N<-1>, N<1>>();
    assert::<N<-1>, N<0>, N<-1>>();

    assert::<N<1>, N<2>, N<-1>>();
    assert::<N<2>, N<1>, N<1>>();

    assert::<N<1>, N<-2>, N<3>>();
    assert::<N<2>, N<-1>, N<3>>();

    // Stable range boundary checks
    assert::<N<LIMIT>, N<{ -LIMIT }>, N<{ 2 * LIMIT }>>();
    assert::<N<{ -LIMIT }>, N<LIMIT>, N<{ -2 * LIMIT }>>();
}

#[test]
fn can_negate_const_numbers() {
    fn assert<N: Neg<Output = O>, O>() {}

    assert::<N<-1>, N<1>>();
    assert::<N<0>, N<0>>();
    assert::<N<1>, N<-1>>();
    assert::<N<LIMIT>, N<{ -LIMIT }>>();
    assert::<N<{ -LIMIT }>, N<LIMIT>>();
}

#[test]
fn num_eq_yields_bit() {
    fn assert<A: NumEq<B, Output = O>, B, O>() {}

    assert::<N<0>, N<0>, typenum::B1>();
    assert::<N<7>, N<7>, typenum::B1>();
    assert::<N<-5>, N<-5>, typenum::B1>();
    assert::<N<3>, N<4>, typenum::B0>();
    assert::<N<-1>, N<1>, typenum::B0>();
    assert::<N<LIMIT>, N<LIMIT>, typenum::B1>();
    assert::<N<LIMIT>, N<{ -LIMIT }>, typenum::B0>();
}

#[test]
fn can_halve_even_exponents_at_both_boundaries() {
    fn assert<A: Halve<Output = O>, O>() {}

    assert::<N<0>, N<0>>();
    assert::<N<LIMIT>, N<{ LIMIT / 2 }>>();
    assert::<N<{ -LIMIT }>, N<{ -LIMIT / 2 }>>();
}

#[test]
fn boundary_results_can_be_reused_when_they_remain_in_range() {
    let _: N<LIMIT> = (N::<LIMIT> + N::<{ -LIMIT }>) + N::<LIMIT>;
    let _: N<{ -LIMIT }> = (N::<{ -LIMIT }> - N::<{ -LIMIT }>) - N::<LIMIT>;
}

#[test]
fn bit_all_folds() {
    fn assert<L: BitAll<Output = O>, O>() {}
    use typenum::{B0, B1};

    assert::<(), B1>();
    assert::<(B1, ()), B1>();
    assert::<(B1, (B1, (B1, ()))), B1>();
    assert::<(B1, (B0, (B1, ()))), B0>();
    assert::<(B0, ()), B0>();
}

#[test]
fn native_results_reuse_the_full_i16_domain() {
    let sum = N::<20000> + N::<10000>;
    let _: N<32767> = sum + N::<2767>;
    let _: N<-32768> = N::<-32760> - N::<8>;
    let _: N<32767> = -N::<-32767>;

    fn half<A: Halve<Output = B>, B>() {}
    half::<N<-32768>, N<-16384>>();
    half::<N<32766>, N<16383>>();
    fn equal<A: NumEq<B, Output = O>, B, O>() {}
    equal::<N<32767>, N<32767>, typenum::B1>();
    equal::<N<-32768>, N<32767>, typenum::B0>();
}
