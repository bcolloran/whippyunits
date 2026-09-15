//! `vqty!`'s per-entry form takes both a length and a unit list, and they must
//! agree. A length that disagrees with the list must fail to compile rather than
//! name a vector whose shape contradicts its units.

use whippyalgebra::nalgebra::vqty;

// Three entries declared, but only two units listed.
type Bad = vqty!(3, [m / s, kg], f64);

fn main() {
    let _ = core::mem::size_of::<Bad>();
}
