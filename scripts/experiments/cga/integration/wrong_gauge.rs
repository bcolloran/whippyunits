use whippyalgebra::dims;
use whippyalgebra::nalgebra::{SMatrix, UniformUnitMatrix};
use whippyunits::unit;
fn main() {
    let matrix = UniformUnitMatrix::<unit!(m / s), SMatrix<f64, 2, 1>>::from_nalgebra(
        SMatrix::<f64, 2, 1>::new(10.0, 20.0));
    let _ = matrix.gauge::<dims![m, m], dims![m]>();
}
