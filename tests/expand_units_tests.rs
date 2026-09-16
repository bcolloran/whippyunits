//! Tests for `#[expand_units]`: built-in derives on *generic* items whose field
//! types spell out `unit!`/`qty!` inline.
//!
//! Without the attribute, every generic definition below is rejected with
//! "`derive` cannot be used on items with type macros" (pinned by
//! `tests/compile_fail_stable/expand_units_below_derive.rs`). With it, the
//! derives see the expanded types and behave exactly as they do for a field
//! typed through a type alias.

// `Quantity<unit!(..), T>` spelled inline trips clippy's type-complexity lint
// whether or not the attribute is involved (the expanded unit type is large).
#![allow(clippy::type_complexity)]

use num_traits::NumCast;
use whippyunits::{Quantity, expand_units, qty, quantity, unit, value};

#[expand_units]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Kinematics<T: Copy + NumCast> {
    pos: Quantity<unit!(m), T>,
    vel: qty!(m / s, T),
}

#[test]
fn derives_on_generic_struct_with_inline_unit_types() {
    let rest = Kinematics::<f64>::default();
    assert_eq!(value!(rest.pos, m), 0.0);
    assert_eq!(value!(rest.vel, m / s), 0.0);

    let moving = Kinematics {
        vel: quantity!(2.0, m / s),
        ..rest
    };
    let copied = moving; // `Copy`
    assert_eq!(copied, moving);
    assert_ne!(copied, rest);
    assert!(format!("{copied:?}").starts_with("Kinematics {"));
}

// The generic parameter may be the brand (third `qty!` argument) rather than
// the storage type.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Mine;

#[expand_units]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Branded<B> {
    len: qty!(m, f32, B),
}

#[test]
fn generic_brand_parameter() {
    let b = Branded::<Mine>::default();
    assert_eq!(b.len, quantity!(0.0, m, f32, Mine));
}

// Macro calls nested inside other generic types are rewritten too.
#[expand_units]
#[derive(Default, Clone, Debug, PartialEq)]
struct Samples<T: Copy + NumCast> {
    readings: Vec<qty!(K, T)>,
    latest: Option<Quantity<unit!(K), T>>,
}

#[test]
fn nested_type_positions_are_expanded() {
    let mut s = Samples::<f64>::default();
    assert!(s.readings.is_empty());
    assert_eq!(s.latest, None);

    s.readings.push(quantity!(300.0, K));
    s.latest = s.readings.last().copied();
    assert_eq!(s.latest, Some(quantity!(300.0, K)));
}

#[expand_units]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reading<T: Copy + NumCast> {
    Length(qty!(m, T)),
    Duration(qty!(s, T)),
    Missing,
}

#[test]
fn derives_on_generic_enum() {
    let r = Reading::Length(quantity!(1.5, m));
    assert_eq!(r.clone(), r);
    assert_ne!(r, Reading::<f64>::Missing);
    assert_ne!(r, Reading::Duration(quantity!(1.5, s)));
}

// Where there is nothing the derives needed help with (a non-generic item),
// the attribute is a harmless no-op.
#[expand_units]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Plain {
    vel: qty!(m / s),
    count: u32,
}

#[test]
fn non_generic_items_are_unaffected() {
    assert_eq!(
        Plain::default(),
        Plain {
            vel: quantity!(0.0, m / s),
            count: 0
        }
    );
}

// The rewrite yields exactly the type the standalone macro would have: a field
// declared under the attribute is assignable to the type-alias spelling.
type Speed<T> = qty!(m / s, T);

#[expand_units]
struct ViaAttribute<T> {
    v: qty!(m / s, T),
}

#[test]
fn expansion_is_the_same_type_as_the_standalone_macro() {
    let x = ViaAttribute::<f64> {
        v: quantity!(4.0, m / s),
    };
    let y: Speed<f64> = x.v;
    assert_eq!(value!(y, m / s), 4.0);
}
