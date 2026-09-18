use cga_probe::{AutoReduce, BitAll, Halve, Mixed, N, NumEq, Uniform, Unit, UnitMul, UnitSqrt};
use std::ops::{Add, Neg, Sub};

fn same<T>(_: T, _: T) {}
fn numeral_equal<A: NumEq<B, Output = O>, B, O>() {}
fn half<A: Halve<Output = O>, O>() {}
fn add<A: Add<B, Output = O>, B, O>() {}
fn sub<A: Sub<B, Output = O>, B, O>() {}
fn neg<A: Neg<Output = O>, O>() {}
fn root<U: UnitSqrt>() { cga_probe::witness_root::<U>(); }
fn generic_product<A: UnitMul<B>, B>() -> std::marker::PhantomData<A::Output> {
    std::marker::PhantomData
}
fn generic_reduce<A: AutoReduce<B>, B>() -> std::marker::PhantomData<A::Output> {
    std::marker::PhantomData
}
fn generic_root<U: UnitSqrt>() -> std::marker::PhantomData<U::Output> {
    std::marker::PhantomData
}

fn main() {
    // Consumer has no nightly attributes: the native implementation lives in
    // another crate, and callers use the existing associated-output API.
    let _: N<5> = cga_probe::explicit_add(N::<2>, N::<3>);
    let _: N<5> = cga_probe::explicit_add(N::<3>, N::<2>);
    let inferred = cga_probe::explicit_add(N::<2>, N::<3>);
    std::hint::black_box(inferred);
    let _: N<-12> = cga_probe::nested(N::<-10>, N::<3>, N::<5>);
    let _: N<30000> = (N::<20000> + N::<10000>) + N::<0>;
    let _: N<-32768> = N::<-32767> - N::<1>;
    let _: N<32767> = -N::<-32767>;
    add::<N<-10>, N<20>, N<10>>();
    sub::<N<-10>, N<20>, N<-30>>();
    neg::<N<-10>, N<10>>();
    half::<N<-32768>, N<-16384>>();
    half::<N<32766>, N<16383>>();
    half::<N<0>, N<0>>();
    numeral_equal::<N<32767>, N<32767>, typenum::B1>();
    numeral_equal::<N<-32768>, N<32767>, typenum::B0>();
    same(std::marker::PhantomData::<Unit<5, -3>>, generic_product::<Unit<2, -1>, Unit<3, -2>>());
    same(std::marker::PhantomData::<Unit<2, -1>>, generic_root::<Unit<4, -2>>());
    root::<Unit<-6, 4>>();
    same(std::marker::PhantomData::<Uniform>, generic_reduce::<Unit<1, 0>, Unit<1, 0>>());
    same(std::marker::PhantomData::<Mixed>, generic_reduce::<Unit<1, 0>, Unit<2, 0>>());
    same(std::marker::PhantomData::<Mixed>, generic_reduce::<Unit<1, 0>, Unit<1, -3>>());
    fn fold<B: BitAll<Output = typenum::B1>>() {}
    fold::<(typenum::B1, (typenum::B1, ()))>();
    assert_eq!(std::mem::size_of::<N<32767>>(), 0);
}
