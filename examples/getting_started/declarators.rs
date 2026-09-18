//! Syntax Options: Choosing the Right Way to Create Quantities
//!
//! WhippyUnits provides three ways to create quantities. This example
//! shows when to use each approach.
//!
//! The type and value macros work with any catalog selection. Method syntax
//! below requires `units-dynamics`; suffix syntax also requires `literals`.
//! Enable `std` for this example's quantity debug output.

use std::fmt::Debug;

use num_traits::NumCast;
// For method syntax (.meters(), .seconds(), etc.)
#[cfg(feature = "units-dynamics")]
use whippyunits::default_declarators::*;

use whippyunits::qty;
// For macro syntax (quantity! macro)
use whippyunits::quantity;

#[whippyunits::expand_units]
#[derive(Default, Clone, Copy, Debug)]
struct Inner<T: Default + Clone + Copy + Debug + NumCast> {
    pub force: qty!(kg * m / s ^ 2, T),
}

#[whippyunits::expand_units]
#[derive(Default, Clone, Copy, Debug)]
struct Test<T: Default + Clone + Copy + Debug + NumCast> {
    pub plain_number: T,
    pub inner: Inner<T>,
    pub vel: qty!(m / s, T),
    pub vel32: qty!(m / s, f32),
    // pub acc: vqty!(m / s ^ 2, T),
}

struct TestNoGeneric {
    pub plain_number: f32,
    pub vel: qty!(m / s),
    pub vel32: qty!(m / s, f32),
}

fn main() {
    println!("WhippyUnits Syntax Options");
    println!("==========================\n");

    let test = Test::<f32>::default();
    println!("Test struct: {:?}", test);
    println!("Inner struct: {:?}", test.inner);

    let test_f64 = Test::<f64>::default();
    println!("Test f64 struct: {:?}", test_f64);
    println!("Inner f64 struct: {:?}", test_f64.inner);

    let concrete = TestNoGeneric {
        plain_number: 1.0,
        vel: quantity!(2.0, m / s),
        vel32: quantity!(3.0, m / s, f32),
    };
    println!(
        "Concrete fields: {:?}, {:?}, {:?}",
        concrete.plain_number, concrete.vel, concrete.vel32
    );

    // ============================================================
    // OPTION 1: Method Syntax (.meters(), .seconds(), etc.)
    // ============================================================
    // Requires: use whippyunits::default_declarators::*;
    // Pros: Familiar Rust syntax, good IDE support
    // Cons: More verbose, requires trait import, limited compound units, non-const

    #[cfg(feature = "units-dynamics")]
    {
        println!("1. Method Syntax (.meters(), .seconds(), etc.)");
        let _distance = 5.0.meters();
        let _mass = 2.5.kilograms();
        let _time = 30.0.seconds();
        println!();
    }

    // ============================================================
    // OPTION 2: Macro Syntax (quantity! macro)
    // ============================================================
    // Requires: use whippyunits::quantity;
    // Pros: Flexible, works everywhere, arbitrary compound units, const
    // Cons: Macro syntax, less familiar

    println!("2. Macro Syntax (quantity! macro)");
    let _distance = quantity!(5.0, m);
    let _mass = quantity!(2.5, kg);
    let _time = quantity!(30.0, s);

    // Can also do calculations in the macro
    let _area = quantity!(5.0 * 4.0, m ^ 2);
    let _velocity = quantity!(100.0 / 10.0, m / s);

    // Best Practice: For compound units, use compound unit literal expressions
    // ✅ Preferred: quantity!(10.0, m / s)
    // ❌ Avoid: quantity!(10.0, m) / quantity!(1.0, s)
    // This provides better rust-analyzer interaction and more reliable constant folding

    #[cfg(all(feature = "literals", feature = "units-dynamics"))]
    literal_syntax();
}

// OPTION 3: Literal Syntax (5.0m, 10s, etc.)
// Pros: Most concise, natural reading, const
// Cons: Requires attribute, only works in annotated scopes, no compound units
// Keep the attribute on a separate, feature-gated function so disabling literals
// also skips their macro expansion.
#[cfg(all(feature = "literals", feature = "units-dynamics"))]
#[culit::culit(whippyunits::default_declarators::literals)]
fn literal_syntax() {
    println!("3. Literal Syntax (5.0m, 10s, etc.)");

    let _distance = 100.0m; // f64 meters
    let _mass = 5kg; // i32 kilograms
    let _time = 10.0s; // f64 seconds
    println!();
}
