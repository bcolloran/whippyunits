use core::ops::Neg;
use whippyunits_core::num::{N, Num};
const VALUE: i16 = <<N<-32768> as Neg>::Output as Num>::N;
fn main() {
    std::hint::black_box(VALUE);
}
