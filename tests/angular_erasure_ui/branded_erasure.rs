use whippyunits::{Quantity, qty};

struct Brand;

fn main() {
    let angle: qty!(rad, f64, Brand) = Quantity::new(2.0);
    let _: f64 = angle.into();

    let compound: qty!(rad * m, f64, Brand) = Quantity::new(2.0);
    let _: qty!(m, f64, Brand) = compound.into();
}
