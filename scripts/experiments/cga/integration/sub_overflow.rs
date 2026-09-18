use core::ops::Sub;
use whippyunits_core::num::{N, Num};
const VALUE: i16 = <<N<-32768> as Sub<N<1>>>::Output as Num>::N;
fn main() {
    std::hint::black_box(VALUE);
}
