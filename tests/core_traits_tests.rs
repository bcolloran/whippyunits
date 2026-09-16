//! Tests for the blanket `Default`/`Eq`/`Hash` impls on `Quantity`.
//!
//! These sit alongside the blanket `Copy` impl in `src/quantity.rs`: they are
//! generic over the phantom `U`/`Brand` parameters and bound only on the storage
//! type, so the main thing worth pinning down is that they are reachable through
//! `#[derive]` on a user struct and that `Hash` agrees with `Eq`.
//!
//! `Eq` and `Hash` are exercised over integer storage because the float storage
//! types do not implement either trait themselves.

use std::collections::{HashMap, HashSet};

use whippyunits::default_declarators::*;
use whippyunits::qty;
use whippyunits::quantity;
use whippyunits::value;

// ============================================================================
// Default
// ============================================================================

#[test]
fn default_is_zero_valued() {
    let d: qty!(m / s) = Default::default();
    assert_eq!(value!(d, m / s), 0.0);
}

#[test]
fn default_is_the_additive_identity() {
    let v = quantity!(5.0, m / s);
    let zero: qty!(m / s) = Default::default();
    assert_eq!(v + zero, v);
}

#[test]
fn default_works_for_integer_storage() {
    let n: qty!(m, i32) = Default::default();
    assert_eq!(value!(n, m, i32), 0);
}

/// The case that motivated these impls: `#[derive(Default)]` on a struct with a
/// quantity field could not resolve `Default` for `Quantity`.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Body {
    vel: qty!(m / s),
    mass: qty!(kg),
}

#[test]
fn default_is_reachable_through_derive() {
    let b = Body::default();
    assert_eq!(value!(b.vel, m / s), 0.0);
    assert_eq!(value!(b.mass, kg), 0.0);
}

#[test]
fn derive_default_composes_with_struct_update_syntax() {
    let b = Body {
        vel: quantity!(3.0, m / s),
        ..Default::default()
    };
    assert_eq!(value!(b.vel, m / s), 3.0);
    assert_eq!(value!(b.mass, kg), 0.0);
}

// ============================================================================
// Eq
// ============================================================================

#[test]
fn eq_holds_for_integer_storage() {
    fn requires_eq<T: Eq>(_: &T) {}

    let a: qty!(m, i32) = 5.meters();
    let b: qty!(m, i32) = 5.meters();
    let c: qty!(m, i32) = 6.meters();

    requires_eq(&a);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn eq_is_reflexive_symmetric_and_transitive() {
    let a: qty!(kg, i32) = 7.kilograms();
    let b: qty!(kg, i32) = 7.kilograms();
    let c: qty!(kg, i32) = 7.kilograms();

    assert_eq!(a, a);
    assert!(a == b && b == a);
    assert!(a == b && b == c && a == c);
}

// ============================================================================
// Hash
// ============================================================================

#[test]
fn equal_quantities_hash_equally() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_of(q: &qty!(m, i32)) -> u64 {
        let mut h = DefaultHasher::new();
        q.hash(&mut h);
        h.finish()
    }

    let a: qty!(m, i32) = 5.meters();
    let b: qty!(m, i32) = 5.meters();

    assert_eq!(a, b);
    assert_eq!(hash_of(&a), hash_of(&b), "Hash must agree with Eq");
}

#[test]
fn quantities_work_as_hash_set_members() {
    let mut seen: HashSet<qty!(m, i32)> = HashSet::new();

    assert!(seen.insert(5.meters()));
    assert!(!seen.insert(5.meters()), "duplicate should not re-insert");
    assert!(seen.insert(6.meters()));

    assert_eq!(seen.len(), 2);
    assert!(seen.contains(&5.meters()));
    assert!(!seen.contains(&7.meters()));
}

#[test]
fn quantities_work_as_hash_map_keys() {
    let mut labels: HashMap<qty!(s, i32), &str> = HashMap::new();
    labels.insert(1.seconds(), "one second");
    labels.insert(60.seconds(), "one minute");

    assert_eq!(labels.get(&1.seconds()), Some(&"one second"));
    assert_eq!(labels.get(&60.seconds()), Some(&"one minute"));
    assert_eq!(labels.get(&2.seconds()), None);
}

/// `Hash` is bound only on the storage type, so it must not drag in bounds the
/// phantom parameters cannot satisfy. Distinct dimensions are distinct types,
/// so this is really a check that each one is independently hashable.
#[test]
fn hash_is_available_across_dimensions() {
    fn requires_hash<T: std::hash::Hash>(_: &T) {}

    let length: qty!(m, i32) = 1.meters();
    let mass: qty!(kg, i32) = 1.kilograms();
    let time: qty!(s, i32) = 1.seconds();

    requires_hash(&length);
    requires_hash(&mass);
    requires_hash(&time);
}
