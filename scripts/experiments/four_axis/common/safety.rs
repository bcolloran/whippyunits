use axis_engine::*;

fn close(actual:f64,expected:f64) {
    assert!((actual-expected).abs() <= 1e-11*expected.abs().max(1.0),"{actual} != {expected}");
}

#[test] fn tick_is_one_sixtieth_of_a_second() {
    let second:Quantity<Second>=Quantity::<Tick>::new(60.0).rescale();
    close(second.raw,1.0);
    let tick:Quantity<Tick>=Quantity::<Millisecond>::new(1000.0).rescale();
    close(tick.raw,60.0);
}
#[test] fn degree_retains_pi_and_angle_axis() {
    let radians:Quantity<Radian>=Quantity::<Degree>::new(180.0).rescale();
    close(radians.raw,core::f64::consts::PI);
    let degrees:Quantity<Degree>=radians.rescale();
    close(degrees.raw,180.0);
}
#[test] fn mixed_scale_dynamics() {
    let got=axis_consumer::dynamics(1000.0,60.0,2000.0,180.0);
    for (actual,expected) in got.into_iter().zip([1.0,1.0,2.0,core::f64::consts::PI,1000.0]) {close(actual,expected)}
    let got=axis_consumer::dynamics(750.0,30.0,3000.0,90.0);
    for (actual,expected) in got.into_iter().zip([1.5,3.0,9.0,core::f64::consts::PI,750.0]) {close(actual,expected)}
}
#[test] fn unit_power_and_matrix_shapes() {
    for (actual,expected) in axis_consumer::uniform_matrix().into_iter().zip([24.0,5.0,120.0]) {close(actual,expected)}
    for (actual,expected) in axis_consumer::mixed_matrix().into_iter().zip([3.5,3.0,0.5]) {close(actual,expected)}
    axis_consumer::type_proofs();
}
#[test] fn zero_cost_layout() {
    assert_eq!(core::mem::size_of::<Quantity<Force>>(),core::mem::size_of::<f64>());
    assert_eq!(core::mem::align_of::<Quantity<Force>>(),core::mem::align_of::<f64>());
}
