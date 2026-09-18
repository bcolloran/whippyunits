//! Exercise the public API with each optional constructor/literal catalog.
//!
//! Keep the DSL and namespace tests unconditional: disabled catalogs must not
//! change unit arithmetic, the eight-axis scale configuration, or branding.

use whippyunits::{Quantity, qty, quantity, unit, value};

whippyunits::define_unit_declarators!(
    local_catalog,
    LocalBrand,
    Kilogram,
    Millimeter,
    Second,
    Ampere,
    Kelvin,
    Mole,
    Candela,
    Radian
);
whippyunits::define_unit_declarators!(branded_catalog, Brand);
whippyunits_proc_macros::define_literals!(custom_literals);

#[test]
fn full_unit_dsl_remains_available_without_catalogs() {
    const ACCELERATION: qty!(m / s ^ 2) = quantity!(9.81, m / s ^ 2);
    let all_axes: Quantity<unit!(kg * m * A * K * mol * cd * rad / s), f64> =
        quantity!(2.0, kg * m * A * K * mol * cd * rad / s);
    let voltage: qty!(V, f32) = quantity!(3.0, V, f32);

    assert_eq!(ACCELERATION.unsafe_value, 9.81);
    assert_eq!(all_axes.unsafe_value, 2.0);
    assert_eq!(voltage.unsafe_value, 3.0);
    assert_eq!(value!(quantity!(2.0, km), m), 2000.0);
}

#[test]
fn nonstorage_and_affine_dsl_do_not_require_declarator_traits() {
    let inches: qty!(cm) = quantity!(2.0, inch);
    let temperature: qty!(K) = quantity!(20.0, celsius);

    assert!((inches.unsafe_value - 5.08).abs() < 1e-12);
    assert!((value!(inches, m) - 0.0508).abs() < 1e-12);
    assert!((temperature.unsafe_value - 293.15).abs() < 1e-12);
    assert!((value!(inches, inch) - 2.0).abs() < 1e-12);
    assert!((value!(temperature, celsius) - 20.0).abs() < 1e-12);
}

#[test]
fn omitted_catalogs_preserve_constructor_inference_and_signed_zero() {
    let inches_i32: qty!(cm, i32) = quantity!(2, inch);
    let inches_i64: qty!(cm, i64) = quantity!(2_i64, inch);
    let celsius: qty!(K) = quantity!(20, celsius);
    let fahrenheit: qty!(degR) = quantity!(32, fahrenheit);
    let negative_zero: qty!(cm) = quantity!(-0.0, inch);

    assert_eq!(inches_i32.unsafe_value, 5);
    assert_eq!(inches_i64.unsafe_value, 5);
    assert!((celsius.unsafe_value - 293.15).abs() < 1e-12);
    assert!((fahrenheit.unsafe_value - 491.7).abs() < 1e-12);
    assert_eq!(negative_zero.unsafe_value.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn local_namespace_keeps_all_scale_parameters_and_brand() {
    let distance: qty!(mm, f64, local_catalog::LocalBrand) = local_catalog::quantity!(2.0, m);
    let current: qty!(A, f64, local_catalog::LocalBrand) = local_catalog::quantity!(3.0, A);

    assert_eq!(distance.unsafe_value, 2000.0);
    assert_eq!(current.unsafe_value, 3.0);
}

#[test]
fn branded_namespace_dsl_is_independent_of_catalog_selection() {
    let distance: qty!(m, f64, branded_catalog::Brand) = branded_catalog::quantity!(2.0, m);
    let temperature: qty!(K, i32, branded_catalog::Brand) = branded_catalog::quantity!(300, K, i32);

    assert_eq!(distance.unsafe_value, 2.0);
    assert_eq!(temperature.unsafe_value, 300);
}

#[cfg(feature = "units-dynamics")]
#[test]
fn dynamics_includes_prefixes_derived_units_and_angles() {
    use whippyunits::default_declarators::*;

    let length: Millimeter = 2.0.millimeters();
    let force: Newton = 3.0.newtons();
    let energy: Joule = 4.0.joules();
    let power: Watt = 5.0.watts();
    let pressure: Pascal = 6.0.pascals();
    let angle: Radian = 7.0.radians();

    assert_eq!(length.unsafe_value, 2.0);
    assert_eq!(force.unsafe_value, 3.0);
    assert_eq!(energy.unsafe_value, 4.0);
    assert_eq!(power.unsafe_value, 5.0);
    assert_eq!(pressure.unsafe_value, 6.0);
    assert_eq!(angle.unsafe_value, 7.0);
}

#[cfg(feature = "units-dynamics")]
#[test]
fn local_and_branded_dynamics_methods_follow_the_catalog() {
    {
        use local_catalog::*;
        let distance: qty!(mm, f64, LocalBrand) = 2.0.meters();
        let angle: qty!(rad, i32, LocalBrand) = 3_i32.radians();
        assert_eq!(distance.unsafe_value, 2000.0);
        assert_eq!(angle.unsafe_value, 3);
    }
    {
        use branded_catalog::*;
        let energy: qty!(J, f64, Brand) = 4.0.joules();
        assert_eq!(energy.unsafe_value, 4.0);
    }
}

#[cfg(feature = "units-electrical")]
#[test]
fn electrical_includes_base_and_derived_constructors() {
    use whippyunits::default_declarators::*;

    let current: Ampere = 2.0.amperes();
    let voltage: Volt = 3.0.volts();
    let capacitance: Farad = 4.0.farads();
    let resistance: Ohm = 5.0.ohms();
    assert_eq!(current.unsafe_value, 2.0);
    assert_eq!(voltage.unsafe_value, 3.0);
    assert_eq!(capacitance.unsafe_value, 4.0);
    assert_eq!(resistance.unsafe_value, 5.0);

    let local_current: qty!(A, f64, local_catalog::LocalBrand) =
        local_catalog::LocalCurrent::amperes(6.0);
    assert_eq!(local_current.unsafe_value, 6.0);
}

#[cfg(feature = "units-thermal")]
#[test]
fn thermal_includes_absolute_and_affine_constructors() {
    use whippyunits::default_declarators::*;

    let temperature: Kelvin = 300.0.kelvin();
    let celsius: Kelvin = 20.0.celsius();
    assert_eq!(temperature.unsafe_value, 300.0);
    assert!((celsius.unsafe_value - 293.15).abs() < 1e-12);
}

#[cfg(feature = "units-chemical")]
#[test]
fn chemical_includes_amount_constructors() {
    use whippyunits::default_declarators::*;

    let amount: Mole = 2.0.moles();
    let small_amount: Millimole = 3.0.millimoles();
    assert_eq!(amount.unsafe_value, 2.0);
    assert_eq!(small_amount.unsafe_value, 3.0);
}

#[cfg(feature = "units-photometric")]
#[test]
fn photometric_includes_illuminance() {
    use whippyunits::default_declarators::*;

    let intensity: Candela = 2.0.candela();
    let illuminance: Lux = 3.0.lux();
    assert_eq!(intensity.unsafe_value, 2.0);
    assert_eq!(illuminance.unsafe_value, 3.0);
}

#[cfg(all(feature = "literals", feature = "units-dynamics"))]
#[culit::culit(whippyunits::default_declarators::literals)]
#[test]
fn default_literals_follow_the_selected_catalog() {
    let distance: qty!(mm) = 2.0mm;
    let energy: qty!(J, i64) = 3J_i64;
    let angle: qty!(rad, f32) = 4.0rad_f32;

    assert_eq!(distance.unsafe_value, 2.0);
    assert_eq!(energy.unsafe_value, 3);
    assert_eq!(angle.unsafe_value, 4.0);
}

#[cfg(all(feature = "literals", feature = "units-dynamics"))]
#[culit::culit(local_catalog::literals)]
#[test]
fn local_literals_preserve_rescaling_and_brand() {
    let distance: qty!(mm, f64, local_catalog::LocalBrand) = 2.0m;
    assert_eq!(distance.unsafe_value, 2000.0);
}

#[cfg(all(feature = "literals", feature = "units-dynamics"))]
#[culit::culit(branded_catalog::literals)]
#[test]
fn branded_literals_preserve_brand() {
    let distance: qty!(m, f64, branded_catalog::Brand) = 2.0m;
    assert_eq!(distance.unsafe_value, 2.0);
}

#[cfg(all(feature = "literals", feature = "units-dynamics"))]
#[culit::culit(custom_literals)]
#[test]
fn custom_literal_generator_obeys_the_same_features() {
    let force: qty!(N) = 2.0N;
    assert_eq!(force.unsafe_value, 2.0);
}

#[cfg(any(not(feature = "units-electrical"), not(feature = "literals")))]
#[test]
fn disabled_catalog_items_are_not_exported() {
    let cases = trybuild::TestCases::new();
    #[cfg(not(feature = "units-electrical"))]
    cases.compile_fail("tests/catalog_ui/absent_electrical.rs");
    #[cfg(any(not(feature = "literals"), not(feature = "units-dynamics")))]
    cases.compile_fail("tests/catalog_ui/absent_literals.rs");
}
