use whippyunits::{Quantity, qty};

fn main() {
    let positive: qty!(rad ^ 10) = Quantity::new(2.0);
    let _: f64 = positive.into();

    let negative: qty!(1 / rad ^ 10) = Quantity::new(2.0);
    let _: f64 = negative.into();

    let compound: qty!(rad ^ 10 * m) = Quantity::new(2.0);
    let _: qty!(m) = compound.into();
}
