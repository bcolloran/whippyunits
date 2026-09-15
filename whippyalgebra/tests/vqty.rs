#![cfg(feature = "nalgebra")]
//! [`vqty!`] names unit-safe column vector types. These tests pin each form's
//! expansion to the type written out by hand, check that an omitted storage type
//! resolves to the caller's `T`, and build values of the named types with the
//! declarative constructors.

use whippyalgebra::dims;
use whippyalgebra::nalgebra::{
    MixedUnitMatrix, SVector, UniformUnitMatrix, mixed_unit_matrix, uniform_unit_matrix, vqty,
};
use whippyunits::{qty, quantity, unit};

// Each function compiles only if the `vqty!` type and the hand-written type are
// the same type.

fn uniform_explicit(v: vqty!(2, m / s, f32)) -> UniformUnitMatrix<unit!(m / s), SVector<f32, 2>> {
    v
}

// Dotted and unspaced unit spellings reach `unit!` as raw tokens, intact.
fn uniform_compound(
    v: vqty!(3, kg.m2 / s2, f64),
) -> UniformUnitMatrix<unit!(kg * m ^ 2 / s ^ 2), SVector<f64, 3>> {
    v
}

fn mixed_explicit(
    v: vqty!(2, [m / s, kg], f64),
) -> MixedUnitMatrix<dims![m / s, kg], dims![1], SVector<f64, 2>> {
    v
}

// Omitting the storage type uses the `T` in scope.

fn uniform_generic<T>(v: vqty!(5, kg)) -> UniformUnitMatrix<unit!(kg), SVector<T, 5>> {
    v
}

fn mixed_generic<T>(
    v: vqty!(2, [m / s, kg]),
) -> MixedUnitMatrix<dims![m / s, kg], dims![1], SVector<T, 2>> {
    v
}

// The uniform form's length may be a const generic parameter.
fn uniform_const_len<T, const N: usize>(
    v: vqty!(N, m, T),
) -> UniformUnitMatrix<unit!(m), SVector<T, N>> {
    v
}

/// A struct generic over its storage type, mixing a scalar quantity field with a
/// vector one.
struct GenericQuantities<T> {
    mass: qty!(kg, T),
    velocity: vqty!(2, m / s, T),
}

impl<T> GenericQuantities<T> {
    fn new(mass: qty!(kg, T), velocity: vqty!(2, m / s, T)) -> Self {
        Self { mass, velocity }
    }
}

#[test]
fn expansions_match_hand_written_types() {
    let _ = (uniform_explicit, uniform_compound, mixed_explicit);
    let _ = (
        uniform_generic::<f64>,
        mixed_generic::<f32>,
        uniform_const_len::<f64, 4>,
    );
}

#[test]
fn constructors_produce_the_named_types() {
    let velocity: vqty!(2, m / s, f64) =
        uniform_unit_matrix![m / s; [quantity!(1.0, m / s)], [quantity!(2.0, m / s)]];
    let second: qty!(m / s) = velocity.get(1, 0);
    assert_eq!(second.unsafe_value, 2.0);

    type State = dims![m / s, kg];
    let state: vqty!(2, [m / s, kg], f64) = mixed_unit_matrix![State, dims![1];
        [quantity!(3.0, m / s)],
        [quantity!(4.0, kg)],
    ];
    let mass: qty!(kg) = state.get::<1, 0>();
    assert_eq!(mass.unsafe_value, 4.0);

    let q = GenericQuantities::new(quantity!(5.0, kg), velocity);
    assert_eq!(q.mass.unsafe_value, 5.0);
    assert_eq!(q.velocity.get(0, 0).unsafe_value, 1.0);
}
