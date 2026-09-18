use whippyunits::{UnitSqrt, unit};
fn require_root<U: UnitSqrt>() {}
fn main() {
    // No root output is evaluated: this bound itself must reject odd dimensions.
    require_root::<unit!(m)>();
}
