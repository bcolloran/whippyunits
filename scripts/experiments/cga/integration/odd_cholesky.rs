use whippyalgebra::nalgebra::{SMatrix, UniformUnitMatrix};
use whippyunits::unit;
fn main() {
    let matrix = UniformUnitMatrix::<unit!(m), SMatrix<f64, 2, 2>>::from_nalgebra(
        SMatrix::<f64, 2, 2>::identity());
    let _ = matrix.cholesky();
}
