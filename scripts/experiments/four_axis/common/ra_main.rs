// Minimal-engine LSP probe. This local adapter matches the measurement harness
// protocol; it does not import the production whippyunits declarator catalog.
mod whippyunits {
    pub mod default_declarators {
        pub trait MetricLength {
            fn meters(self) -> axis_engine::Quantity<axis_engine::Meter>;
        }

        impl MetricLength for f64 {
            fn meters(self) -> axis_engine::Quantity<axis_engine::Meter> {
                axis_engine::Quantity::new(self)
            }
        }
    }
}

use axis_engine::*;
#[allow(unused_imports)]
use whippyunits::default_declarators::*;

fn main() {
    let distance = Quantity::<Millimeter>::new(3000.0).rescale::<Meter>();
    let duration = Quantity::<Tick>::new(120.0).rescale::<Second>();
    let velocity = distance / duration; // benchmark-hover

    let acceleration = velocity / duration;
    let mass = Quantity::<Gram>::new(2000.0).rescale::<Kilogram>();
    let force: Quantity<Force> = mass * acceleration;
    let angle = Quantity::<Degree>::new(90.0).rescale::<Radian>();
    let angular_velocity: Quantity<AngularVelocity> = angle / duration;
    let energy = force * distance;

    let scalar: f64 = 1.0;
    let probe = whippyunits::default_declarators::MetricLength::meters(scalar); // benchmark-completion

    let dynamics = axis_consumer::dynamics(3000.0, 120.0, 2000.0, 90.0);
    let uniform_matrix = axis_consumer::uniform_matrix();
    let mixed_matrix = axis_consumer::mixed_matrix();
    let _ = (velocity, energy, angular_velocity, probe, dynamics, uniform_matrix, mixed_matrix);
}
