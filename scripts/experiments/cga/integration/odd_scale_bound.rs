use whippyunits::{UnitSqrt, unit};
fn require_root<U: UnitSqrt>() {}
fn main() {
    // Length exponent is even, but the base-2 and base-5 scale exponents are -3.
    require_root::<unit!(mm * m)>();
}
