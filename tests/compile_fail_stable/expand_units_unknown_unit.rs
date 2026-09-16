// A bad unit expression inside a field is left for the standalone macro to
// report at the offending tokens, the same way it would without the attribute.
#[whippyunits::expand_units]
struct Wrong<T> {
    vel: whippyunits::qty!(xyz, T),
    raw: T,
}

fn main() {}
