// `#[expand_units]` must be written *above* the derives it is meant to help:
// attributes expand top to bottom, so here `derive` still sees the macro call.
#[derive(Clone)]
#[whippyunits::expand_units]
struct Wrong<T> {
    vel: whippyunits::Quantity<whippyunits::unit!(m / s), T>,
}

fn main() {}
