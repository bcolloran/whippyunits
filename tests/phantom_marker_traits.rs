//! Value traits must not require phantom markers to implement Clone or Copy.

use whippyunits::{Quantity, unit};

// These markers intentionally implement neither Clone nor PartialEq.
struct OpaqueUnit;
struct OpaqueBrand;

// No marker bounds: verify the trait implementations are available even while
// the unit and brand remain unknown to the caller.
fn copy_twice<U, Brand>(quantity: Quantity<U, u32, Brand>) -> [Quantity<U, u32, Brand>; 2] {
    [quantity, quantity]
}

fn clone_storage<U, T: Clone, Brand>(quantity: &Quantity<U, T, Brand>) -> Quantity<U, T, Brand> {
    quantity.clone()
}

#[test]
fn opaque_markers_allow_copy_and_clone() {
    let quantity = Quantity::<OpaqueUnit, u32, OpaqueBrand>::from_raw_value(7);
    let copies = copy_twice(quantity);
    assert_eq!(quantity.unsafe_value, 7);
    assert_eq!(copies.map(|q| q.unsafe_value), [7, 7]);
    assert_eq!(clone_storage(&quantity).unsafe_value, 7);
}

#[test]
fn clone_only_storage_keeps_its_clone_behavior() {
    let mut original = Quantity::<OpaqueUnit, Vec<u32>, OpaqueBrand>::from_raw_value(vec![1, 2]);
    let cloned = clone_storage(&original);
    original.unsafe_value.push(3);
    assert_eq!(original.unsafe_value, [1, 2, 3]);
    assert_eq!(cloned.unsafe_value, [1, 2]);
}

#[test]
fn a_brand_without_clone_can_be_used_in_repeated_arithmetic() {
    type Length = Quantity<unit!(m), i32, OpaqueBrand>;
    let length = Length::from_raw_value(3);
    let sum = length + length;
    assert_eq!(sum.unsafe_value, 6);
    assert_eq!(length.unsafe_value, 3);
}

#[test]
fn pod_views_preserve_storage_with_opaque_markers() {
    fn require_pod<U: 'static, Brand: 'static>() {
        fn check<T: bytemuck::Pod>() {}
        check::<Quantity<U, u32, Brand>>();
    }
    require_pod::<OpaqueUnit, OpaqueBrand>();

    type Q = Quantity<OpaqueUnit, u32, OpaqueBrand>;
    let mut raw = [3_u32, 5];
    let quantities: &mut [Q] = bytemuck::cast_slice_mut(&mut raw);
    assert_eq!(quantities[0].unsafe_value, 3);
    quantities[1].unsafe_value = 8;
    assert_eq!(raw, [3, 8]);
    assert_eq!(core::mem::size_of::<Q>(), core::mem::size_of::<u32>());
    assert_eq!(core::mem::align_of::<Q>(), core::mem::align_of::<u32>());
}

#[test]
fn derived_partial_eq_still_allows_constant_patterns() {
    type Length = Quantity<unit!(m), i32>;
    const ONE: Length = Length::from_raw_value(1);
    let value = Length::from_raw_value(1);
    assert!(matches!(value, ONE));
    assert!(!matches!(Length::from_raw_value(2), ONE));
}
