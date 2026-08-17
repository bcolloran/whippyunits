//! Quantity arithmetic over storage types this crate has never heard of.
//!
//! The point of making the op impls storage-generic is that a downstream
//! numeric type — an autodiff dual, an interval, a fixed-point value — can be used
//! as quantity storage without this crate being changed. These tests pin that
//! property, and in particular pin the *granularity* of it: a storage type that is
//! missing one operation must lose only that operation, not the whole arithmetic
//! surface. (Requiring the full surface up front would mean a type with no
//! meaningful remainder — normal for intervals and dual numbers — could not be used
//! for quantity addition either.) There is no global bound at all, not even `Copy`
//! or `PartialEq`: nothing generated copies or compares the stored value, so a
//! heap-backed or arbitrary-precision scalar is not excluded either.
//!
//! The negative half of that property — that `%` on a `Rem`-less storage type is
//! still correctly rejected — is not asserted here, because a `trybuild` case would
//! need a separate `.stderr` baseline for the stable and `cge` configurations.

use whippyunits::unit;

/// A storage type with the ordinary arithmetic operations but deliberately **no**
/// `Rem`/`RemAssign`, mimicking interval and dual-number types.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct NoRem(f64);

macro_rules! impl_binop {
    ($Trait:ident, $method:ident, $op:tt, $TraitAssign:ident, $method_assign:ident, $op_assign:tt) => {
        impl core::ops::$Trait for NoRem {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                NoRem(self.0 $op rhs.0)
            }
        }
        impl core::ops::$TraitAssign for NoRem {
            fn $method_assign(&mut self, rhs: Self) {
                self.0 $op_assign rhs.0;
            }
        }
    };
}

impl_binop!(Add, add, +, AddAssign, add_assign, +=);
impl_binop!(Sub, sub, -, SubAssign, sub_assign, -=);
impl_binop!(Mul, mul, *, MulAssign, mul_assign, *=);
impl_binop!(Div, div, /, DivAssign, div_assign, /=);

impl core::ops::Neg for NoRem {
    type Output = Self;
    fn neg(self) -> Self {
        NoRem(-self.0)
    }
}

#[test]
fn dimension_preserving_ops_work_for_a_custom_storage_type() {
    let a: unit!(m, NoRem) = <unit!(m, NoRem)>::from_raw_value(NoRem(3.0));
    let b: unit!(m, NoRem) = <unit!(m, NoRem)>::from_raw_value(NoRem(4.0));

    assert_eq!((a + b).unsafe_value, NoRem(7.0));
    assert_eq!((b - a).unsafe_value, NoRem(1.0));
    assert_eq!((-a).unsafe_value, NoRem(-3.0));
    assert!(b > a);

    let mut acc = a;
    acc += b;
    assert_eq!(acc.unsafe_value, NoRem(7.0));
}

#[test]
fn quantity_scalar_ops_work_for_a_custom_storage_type() {
    let a: unit!(m, NoRem) = <unit!(m, NoRem)>::from_raw_value(NoRem(3.0));

    assert_eq!((a * NoRem(2.0)).unsafe_value, NoRem(6.0));
    assert_eq!((a / NoRem(2.0)).unsafe_value, NoRem(1.5));
}

#[test]
fn dimension_combining_ops_work_for_a_custom_storage_type() {
    let d: unit!(m, NoRem) = <unit!(m, NoRem)>::from_raw_value(NoRem(10.0));
    let t: unit!(s, NoRem) = <unit!(s, NoRem)>::from_raw_value(NoRem(2.0));

    // The dimensions combine exactly as they do for the built-in storage types;
    // the annotation is what makes this a dimensional assertion and not just math.
    let v: unit!(m / s, NoRem) = d / t;
    assert_eq!(v.unsafe_value, NoRem(5.0));

    let area: unit!(m ^ 2, NoRem) = d * d;
    assert_eq!(area.unsafe_value, NoRem(100.0));
}

/// A storage type that is neither `Copy` nor `PartialEq` — the shape of a
/// heap-backed or arbitrary-precision scalar — and implements only `Add`.
#[derive(Clone, Debug)]
struct HeapBacked(Vec<u8>, f64);

impl core::ops::Add for HeapBacked {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        HeapBacked(self.0, self.1 + rhs.1)
    }
}

#[test]
fn a_non_copy_storage_type_can_still_add() {
    let a = <unit!(m, HeapBacked)>::from_raw_value(HeapBacked(vec![1], 3.0));
    let b = <unit!(m, HeapBacked)>::from_raw_value(HeapBacked(vec![2], 4.0));
    assert_eq!((a + b).unsafe_value.1, 7.0);
}
