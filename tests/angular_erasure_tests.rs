use whippyunits::{Quantity, qty, quantity, rescale};

#[test]
fn all_supported_nonzero_exponents_erase() {
    macro_rules! check_exponent {
        ($($angle:tt)*) => {{
            let angle: qty!($($angle)*) = Quantity::new(2.5);
            let scalar: f64 = angle.into();
            assert_eq!(scalar, 2.5);

            let compound: qty!(m * ($($angle)*) / s^2) = Quantity::new(3.5);
            let acceleration: qty!(m / s^2) = compound.into();
            assert_eq!(acceleration.unsafe_value, 3.5);
        }};
    }

    check_exponent!(rad);
    check_exponent!(rad ^ 2);
    check_exponent!(rad ^ 3);
    check_exponent!(rad ^ 4);
    check_exponent!(rad ^ 5);
    check_exponent!(rad ^ 6);
    check_exponent!(rad ^ 7);
    check_exponent!(rad ^ 8);
    check_exponent!(rad ^ 9);
    check_exponent!(1 / rad);
    check_exponent!(1 / rad ^ 2);
    check_exponent!(1 / rad ^ 3);
    check_exponent!(1 / rad ^ 4);
    check_exponent!(1 / rad ^ 5);
    check_exponent!(1 / rad ^ 6);
    check_exponent!(1 / rad ^ 7);
    check_exponent!(1 / rad ^ 8);
    check_exponent!(1 / rad ^ 9);
}

#[test]
fn same_storage_erasure_preserves_exact_values() {
    macro_rules! check_storage {
        ($storage:ty, $value:expr) => {{
            let value: $storage = $value;
            let angle: qty!(rad, $storage) = Quantity::new(value);
            let scalar: $storage = angle.into();
            assert_eq!(scalar, value);

            let compound: qty!(m * rad, $storage) = Quantity::new(value);
            let length: qty!(m, $storage) = compound.into();
            assert_eq!(length.unsafe_value, value);
        }};
    }

    check_storage!(f32, 1.25);
    check_storage!(f64, 1.25);
    check_storage!(i16, i16::MAX);
    check_storage!(i32, i32::MAX);
    check_storage!(i64, i64::MAX);
    check_storage!(i128, i128::MAX);
    check_storage!(isize, isize::MAX);

    let negative_zero: f64 = quantity!(-0.0, rad).into();
    assert_eq!(negative_zero.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn scalar_erasure_rescales_before_casting() {
    let radians: f64 = quantity!(90.0, deg).into();
    assert_eq!(radians, core::f64::consts::FRAC_PI_2);

    macro_rules! check_cross_type {
        ($source:ty, $target:ty) => {{
            let raw = 12345 as $source;
            let unscaled: qty!(rad, $source) = Quantity::new(raw);
            let scalar: $target = unscaled.into();
            assert_eq!(scalar, (raw as f64) as $target);

            let scaled: qty!(mrad, $source) = Quantity::new(raw);
            let scalar: $target = scaled.into();
            let float_scaled: qty!(mrad) = Quantity::new(raw as f64);
            let expected = rescale!(float_scaled, rad).unsafe_value as $target;
            assert_eq!(scalar, expected);
        }};
    }

    check_cross_type!(f32, f64);
    check_cross_type!(f64, f32);
    check_cross_type!(i32, f32);
    check_cross_type!(i32, f64);
    check_cross_type!(i64, f32);
    check_cross_type!(i64, f64);
    check_cross_type!(f32, i32);
    check_cross_type!(f64, i32);
    check_cross_type!(f32, i64);
    check_cross_type!(f64, i64);

    // Same-type integer erasure uses integer rescaling, even above f64's
    // exact-integer range; cross-type erasure intentionally goes through f64.
    let large = 9_007_199_254_740_993_i64;
    let scaled: qty!(krad, i64) = Quantity::new(large);
    let scalar: i64 = scaled.into();
    assert_eq!(scalar, large * 1000);
}

#[test]
fn compound_erasure_preserves_the_original_scale() {
    let angular: qty!(deg * m / s ^ 2, i64) = Quantity::new(42);
    let erased: qty!((deg / rad) * m / s ^ 2, i64) = angular.into();
    assert_eq!(erased.unsafe_value, 42);
}

#[test]
fn zero_exponent_keeps_dimensionless_and_identity_conversions() {
    let dimensionless: qty!(1, i64) = Quantity::new(i64::MAX);
    let scalar: i64 = dimensionless.into();
    assert_eq!(scalar, i64::MAX);

    struct Brand;
    let length: qty!(m, i8, Brand) = Quantity::new(42);
    let unchanged: qty!(m, i8, Brand) = length.into();
    assert_eq!(unchanged.unsafe_value, 42);
}

#[test]
fn unsupported_angular_conversions_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/angular_erasure_ui/*.rs");
}
