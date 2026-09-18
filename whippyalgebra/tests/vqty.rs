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

use num_traits::NumCast;

// Each form must be expanded before derives inspect a generic field type.
// The qualified paths also exercise both public locations of the macro.
#[whippyunits::expand_units]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct DerivedQuantities<T: Copy + NumCast + PartialEq + std::fmt::Debug + 'static> {
    mass: qty!(kg, T),
    velocity: vqty!(2, m / s, T),
    masses: vqty!(2, kg),
    state: vqty!(2, [m / s, kg], T),
    default_state: vqty!(2, [m / s, kg]),
    nested: Option<whippyalgebra::nalgebra::vqty!(2, km / h, T,)>,
    root_path: whippyalgebra::vqty!(2, m / s, T),
}

#[whippyunits::expand_units]
#[derive(Clone, Copy, Debug, PartialEq)]
struct ConstLength<T, const N: usize> {
    velocity: vqty!(N, m / s, T),
}

#[test]
fn derives_work_with_vector_type_macros() {
    let rest = DerivedQuantities::<f64>::default();
    let moving = DerivedQuantities {
        velocity: uniform_unit_matrix![m / s;
            [quantity!(1.0, m / s)], [quantity!(2.0, m / s)],
        ],
        state: mixed_unit_matrix![dims![m / s, kg], dims![1];
            [quantity!(3.0, m / s)], [quantity!(4.0, kg)],
        ],
        ..rest
    };
    let copied = moving;
    assert_eq!(copied, moving);
    assert_ne!(moving, rest);
    assert!(format!("{copied:?}").starts_with("DerivedQuantities {"));
    assert_eq!(moving.velocity.get(1, 0).unsafe_value, 2.0);
    assert_eq!(moving.state.get::<1, 0>().unsafe_value, 4.0);

    // Attribute expansion and standalone expansion must agree exactly.
    let _: vqty!(2, kg, f64) = moving.masses;
    let _: vqty!(2, [m / s, kg], f64) = moving.default_state;
    let _: Option<vqty!(2, km / h, f64)> = moving.nested;
    let _: vqty!(2, m / s, f64) = moving.root_path;

    let generic_length = ConstLength {
        velocity: moving.velocity,
    };
    assert_eq!(generic_length, generic_length.clone());
}

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
