use whippyunits::{Quantity, qty};

fn main() {
    let angle: qty!(rad, i8) = Quantity::new(2);
    let _: i8 = angle.into();

    let compound: qty!(rad * m, i8) = Quantity::new(2);
    let _: qty!(m, i8) = compound.into();

    let cross_type: qty!(rad, i16) = Quantity::new(2);
    let _: f64 = cross_type.into();
}
