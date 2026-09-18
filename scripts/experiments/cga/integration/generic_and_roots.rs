//! Associated-output inference, existing boolean identity, and exact roots.
use core::marker::PhantomData;
use core::ops::{Add, Neg, Sub};
use whippyalgebra::dims;
use whippyalgebra::nalgebra::{MixedUnitMatrix, SMatrix, UniformUnitMatrix};
use whippyunits::unit::{MulUnit, SqrtUnit, UnitEq, UnitMul, UnitSqrt};
use whippyunits::{qty, unit};
use whippyunits_core::num::{BitAll, Halve, N, NumEq};

fn add<const A: i16, const B: i16, const O: i16>(a: N<A>, b: N<B>) -> N<O>
where N<A>: Add<N<B>, Output = N<O>> { a + b }
fn sub<const A: i16, const B: i16, const O: i16>(a: N<A>, b: N<B>) -> N<O>
where N<A>: Sub<N<B>, Output = N<O>> { a - b }
fn neg<const A: i16, const O: i16>(a: N<A>) -> N<O>
where N<A>: Neg<Output = N<O>> { -a }
fn product<U: UnitMul<V>, V>() -> PhantomData<MulUnit<U, V>> { PhantomData }
fn root<U: UnitSqrt>() -> PhantomData<SqrtUnit<U>> { PhantomData }
fn same<T>(_: T, _: T) {}
fn eq<A: NumEq<B, Output = Bit>, B, Bit>() {}
fn unit_eq<A: UnitEq<B, Output = Bit>, B, Bit>() {}
fn halve<A: Halve<Output = O>, O>() {}
fn folded<B: BitAll<Output = typenum::B1>>() {}

fn main() {
    let _: N<5> = add(N::<2>, N::<3>);
    let _: N<-1> = sub(N::<2>, N::<3>);
    let _: N<-3> = neg(N::<3>);
    std::hint::black_box(add(N::<-2>, N::<3>));
    eq::<N<2>, N<2>, typenum::B1>();
    eq::<N<-2>, N<2>, typenum::B0>();
    unit_eq::<unit!(m), unit!(m), typenum::B1>();
    unit_eq::<unit!(mm), unit!(m), typenum::B0>();
    unit_eq::<unit!(m), unit!(s), typenum::B0>();
    folded::<(typenum::B1, (typenum::B1, ()))>();
    halve::<N<-6>, N<-3>>();
    same(PhantomData::<unit!(m / tick)>, product::<unit!(m), unit!(1 / tick)>());
    same(PhantomData::<unit!(mm)>, root::<unit!(mm ^ 2)>());
    same(PhantomData::<unit!(1 / tick)>, root::<unit!(1 / tick ^ 2)>());

    type M2 = SMatrix<f64, 2, 2>;
    let matrix: UniformUnitMatrix<unit!(m ^ 2), M2> =
        MixedUnitMatrix::<dims![m, m], dims![1 / m, 1 / m], M2>::new(
            M2::new(4.0, 2.0, 2.0, 4.0)).into_uniform();
    let lower = matrix.cholesky().expect("positive definite").l;
    let entry: qty!(m) = lower.get(0, 0);
    assert_eq!(entry.unsafe_value, 2.0);
    let reconstructed: UniformUnitMatrix<unit!(m ^ 2), _> = lower * lower.transpose();
    assert!((reconstructed.get(0, 1).unsafe_value - 2.0).abs() < 1e-12);
}
