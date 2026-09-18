use whippyalgebra::nalgebra::vqty;

// The attribute must preserve the standalone macro's vector shape check.
#[whippyunits::expand_units]
type Bad = vqty!(3, [m / s, kg], f64);

fn main() {
    let _ = core::mem::size_of::<Bad>();
}
