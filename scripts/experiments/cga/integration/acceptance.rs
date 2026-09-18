//! Real quantities, exact conversions, matrix inference, and valid gauge choice.
use whippyalgebra::dims;
use whippyalgebra::nalgebra::{MixedUnitMatrix, SMatrix, UniformUnitMatrix, rescale_uniform_matrix};
use whippyunits::default_declarators::*;
use whippyunits::{qty, quantity, rescale, unit};

fn main() {
    let duration = 2.0_f64.ticks();
    let velocity = quantity!(3.0, m) / duration;
    let acceleration = velocity / duration;
    let force: qty!(kg * m / tick ^ 2) = quantity!(2.0, kg) * acceleration;
    let energy: qty!(kg * m ^ 2 / tick ^ 2) = force * quantity!(3.0, m);
    assert_eq!(rescale!(60.0_f64.ticks(), s).unsafe_value, 1.0);
    assert_eq!(rescale!(velocity, m / s).unsafe_value, 90.0);
    assert_eq!(rescale!(acceleration, m / s ^ 2).unsafe_value, 2700.0);
    assert_eq!(rescale!(energy, J).unsafe_value, 16200.0);
    assert!((rescale!(90.0_f64.degrees(), rad).unsafe_value
        - core::f64::consts::FRAC_PI_2).abs() < 1e-12);

    type Raw = SMatrix<f64, 3, 3>;
    type PerTick = MixedUnitMatrix<dims![m, m, m], dims![tick, tick, tick], Raw>;
    let matrix: UniformUnitMatrix<unit!(m / tick), Raw> =
        PerTick::new(Raw::identity()).into_uniform();
    let si: UniformUnitMatrix<unit!(m / s), Raw> = rescale_uniform_matrix(&matrix);
    let squared: qty!(m ^ 2 / tick ^ 2) = matrix.dot(&matrix);
    let determinant: qty!(m ^ 3 / tick ^ 3) = matrix.determinant();
    assert_eq!(si.get(0, 0).unsafe_value, 60.0);
    assert_eq!(squared.unsafe_value, 3.0);
    assert_eq!(determinant.unsafe_value, 1.0);
    assert!((matrix.norm().unsafe_value - 3.0_f64.sqrt()).abs() < 1e-12);
    let gauged: PerTick = matrix.gauge::<dims![m, m, m], dims![tick, tick, tick]>();
    let entry: qty!(m / tick) = gauged.get::<0, 0>();
    assert_eq!(entry.unsafe_value, 1.0);
}
