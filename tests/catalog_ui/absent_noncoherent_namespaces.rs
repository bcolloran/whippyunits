#![allow(unused_imports)]

whippyunits::define_unit_declarators!(
    local_units,
    Kilogram,
    Meter,
    Second,
    Ampere,
    Kelvin,
    Mole,
    Candela,
    Radian
);
whippyunits::define_unit_declarators!(branded_units, Brand);

fn main() {
    {
        use local_units::*;
        let _ = 1.0_f64.millimeters();
    }
    {
        use branded_units::*;
        let _ = 1.0_f64.grams();
    }
    {
        use whippyunits::default_declarators::*;
        let _ = 1.0_f64.feet();
    }
}
