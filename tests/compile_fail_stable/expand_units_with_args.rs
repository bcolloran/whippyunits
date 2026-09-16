use whippyunits::expand_units;

#[expand_units(recursive)]
struct Wrong<T> {
    x: T,
}

fn main() {}
