use core::ops::Add;
use whippyunits_core::num::{N, Num};
// A trait bound alone is weaker: force evaluation of the associated output.
const VALUE: i16 = <<N<32767> as Add<N<1>>>::Output as Num>::N;
fn main() {
    std::hint::black_box(VALUE);
}
