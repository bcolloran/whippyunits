use whippyalgebra::dims;
use whippyalgebra::nalgebra::{MixedUnitMatrix, SMatrix};
use whippyunits::qty;
struct FrameA;
struct FrameB;
fn main() {
    let matrix = MixedUnitMatrix::<dims![m], dims![s], SMatrix<f64, 1, 1>, FrameA>::new(
        SMatrix::<f64, 1, 1>::new(2.0));
    let _: qty!(m / s, f64, FrameB) = matrix.get::<0, 0>();
}
