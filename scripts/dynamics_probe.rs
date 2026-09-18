// Shared compile/editor workload. The three benchmark markers and the fully
// qualified meters() expression are consumed by measure_rust_analyzer.py.
use whippyalgebra::dims;
use whippyalgebra::nalgebra::{MixedUnitMatrix, SMatrix, UniformUnitMatrix, rescale_uniform_matrix};
use whippyunits::default_declarators::*;
use whippyunits::{qty, quantity, rescale, unit};

fn matrix_workload() {
    type Raw = SMatrix<f64, 3, 3>;
    type PerTick = MixedUnitMatrix<dims![m, m, m], dims![tick, tick, tick], Raw>;
    let matrix: UniformUnitMatrix<unit!(m / tick), Raw> =
        PerTick::new(Raw::identity()).into_uniform();
    let si: UniformUnitMatrix<unit!(m / s), Raw> = rescale_uniform_matrix(&matrix);
    let squared: qty!(m ^ 2 / tick ^ 2) = matrix.dot(&matrix);
    let determinant: qty!(m ^ 3 / tick ^ 3) = matrix.determinant();
    let norm: qty!(m / tick) = matrix.norm();
    assert_eq!(si.get(0, 0).unsafe_value, 60.0);
    assert_eq!(squared.unsafe_value, 3.0);
    assert_eq!(determinant.unsafe_value, 1.0);
    assert!((norm.unsafe_value - 3.0_f64.sqrt()).abs() < 1e-12);
}

fn main() {
    let distance = quantity!(3.0, m);
    let duration = 2.0_f64.ticks();
    let velocity = distance / duration; // benchmark-hover
    let acceleration = velocity / duration;
    let force = quantity!(2.0, kg) * acceleration;
    let energy = force * distance;
    let angle = 90.0_f64.degrees();
    let angular_velocity = angle / duration;
    let angular_acceleration = angular_velocity / duration;
    let scalar: f64 = 1.0;
    let probe = whippyunits::default_declarators::MetricLength::meters(scalar); // benchmark-completion
    assert_eq!(rescale!(60.0_f64.ticks(), s).unsafe_value, 1.0);
    assert_eq!(rescale!(velocity, m / s).unsafe_value, 90.0);
    assert_eq!(rescale!(acceleration, m / s ^ 2).unsafe_value, 2700.0);
    assert!((rescale!(angle, rad).unsafe_value - core::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!((rescale!(angular_acceleration, rad / s ^ 2).unsafe_value
        - 450.0 * core::f64::consts::PI).abs() < 1e-10);
    let _ = (energy, probe);
    matrix_workload();
}
