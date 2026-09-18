//! Keep dimensional and scale arithmetic working at the reduced-profile boundary.
//! The wide case additionally verifies the public crate forwards its feature.

use whippyunits::{qty, quantity};

#[test]
fn dimension_arithmetic_supports_both_reduced_boundaries_and_doubled_outputs() {
    let positive: qty!(m ^ 32) = quantity!(2.0, m ^ 32);
    let negative: qty!(m ^ -32) = quantity!(3.0, m ^ -32);
    let doubled: qty!(m ^ 64) = positive * positive;
    let inverse_doubled: qty!(m ^ -64) = negative * negative;
    let quotient: qty!(m ^ 64) = positive / negative;
    let cancellation: qty!(1) = positive * negative;

    assert_eq!(doubled.unsafe_value, 4.0);
    assert_eq!(inverse_doubled.unsafe_value, 9.0);
    assert_eq!(quotient.unsafe_value, 2.0 / 3.0);
    assert_eq!(cancellation.unsafe_value, 6.0);
}

#[test]
fn scale_arithmetic_uses_the_same_reduced_input_and_output_ranges() {
    // Hectometers contribute 2 to both prime-2 and prime-5 scale exponents.
    let positive: qty!(hm ^ 16) = quantity!(2.0, hm ^ 16);
    let negative: qty!(hm ^ -16) = quantity!(3.0, hm ^ -16);
    let doubled: qty!(hm ^ 32) = positive * positive;
    let inverse_doubled: qty!(hm ^ -32) = negative * negative;
    let cancellation: qty!(1) = positive * negative;

    assert_eq!(doubled.unsafe_value, 4.0);
    assert_eq!(inverse_doubled.unsafe_value, 9.0);
    assert_eq!(cancellation.unsafe_value, 6.0);
}

#[cfg(feature = "wide-exponents")]
#[test]
fn wide_feature_preserves_the_original_dimension_boundaries() {
    let positive: qty!(m ^ 200) = quantity!(2.0, m ^ 200);
    let negative: qty!(m ^ -200) = quantity!(3.0, m ^ -200);
    let doubled: qty!(m ^ 400) = positive * positive;
    let inverse_doubled: qty!(m ^ -400) = negative * negative;
    let cancellation: qty!(1) = positive * negative;

    assert_eq!(doubled.unsafe_value, 4.0);
    assert_eq!(inverse_doubled.unsafe_value, 9.0);
    assert_eq!(cancellation.unsafe_value, 6.0);
}

#[cfg(feature = "nightly-gca")]
#[test]
fn native_dimension_outputs_can_be_reused_beyond_the_stable_bridge() {
    let a: qty!(m ^ 300) = quantity!(2.0, m ^ 300);
    let b: qty!(m ^ 300) = quantity!(3.0, m ^ 300);
    let product = a * b;
    let restored: qty!(m ^ 300) = product / b;
    assert_eq!(restored.unsafe_value, 2.0);
}
