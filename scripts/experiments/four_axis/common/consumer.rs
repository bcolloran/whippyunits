use axis_engine::*;
use axis_engine::dims::{DCons,DNil};
use axis_engine::uniformity::IsUniform;
use nalgebra::{SMatrix,Vector3};

// Identical body and inference in each workspace. Every intermediate quantity
// keeps its scale, and explicit rescale checks the intended target dimensions.
pub fn dynamics(distance_mm:f64, ticks:f64, mass_g:f64, degrees:f64)->[f64;5] {
    let x=Quantity::<Millimeter>::new(distance_mm);
    let dt=Quantity::<Tick>::new(ticks);
    let m=Quantity::<Gram>::new(mass_g);
    let angle=Quantity::<Degree>::new(degrees);
    let v=x/dt;
    let a=v/dt;
    let f=m*a;
    let omega=angle/dt;
    let v_si:Quantity<Velocity>=v.rescale();
    let a_si:Quantity<Acceleration>=a.rescale();
    let f_si:Quantity<Force>=f.rescale();
    let omega_si:Quantity<AngularVelocity>=omega.rescale();
    let recovered:Quantity<Millimeter>=(v*dt).rescale();
    [v_si.raw,a_si.raw,f_si.raw,omega_si.raw,recovered.raw]
}

pub fn uniform_matrix()->[f64;3] {
    let v=Uniform::<Velocity,3,3>::new(SMatrix::<f64,3,3>::from_diagonal(&Vector3::new(2.0,3.0,4.0)));
    let dt=Uniform::<Tick,3,3>::new(SMatrix::<f64,3,3>::identity()*60.0);
    let position=v*dt;
    let determinant=position.determinant();
    let volume:Quantity<index::PowUnit<Meter,typenum::U3>>=determinant.rescale();
    let square=Quantity::<Meter>::new(5.0)*Quantity::<Meter>::new(5.0);
    let length:Quantity<Meter>=square.sqrt();
    [volume.raw,length.raw,position.raw[(0,0)]]
}

pub type State=DCons<Meter,DCons<Velocity,DNil>>;
pub type Inputs=DCons<One,DNil>;
pub fn mixed_matrix()->[f64;3] {
    let transition=Mixed::<State,State,2,2>::new(SMatrix::<f64,2,2>::new(1.0,0.5,0.0,1.0));
    let state=Mixed::<State,Inputs,2,1>::new(SMatrix::<f64,2,1>::new(2.0,3.0));
    let duration:Quantity<Second>=transition.entry::<0,1>();
    let next=transition*state;
    let position:Quantity<Meter>=next.entry::<0,0>();
    let velocity:Quantity<Velocity>=next.entry::<1,0>();
    [position.raw,velocity.raw,duration.raw]
}

pub fn type_proofs() {
    fn same<T:UnitEq<U,Output=typenum::B1>,U>() {}
    fn different<T:UnitEq<U,Output=typenum::B0>,U>() {}
    fn uniform<T:IsUniform<Out=typenum::B1>>() {}
    fn mixed<T:IsUniform<Out=typenum::B0>>() {}
    same::<MulUnit<Velocity,Second>,Meter>();
    same::<MulUnit<Force,Second>,MulUnit<Kilogram,Velocity>>();
    different::<Degree,Radian>();
    different::<Meter,Second>();
    uniform::<index::Repeated<Meter,6>>();
    mixed::<State>();
}

pub fn run()->f64 {
    type_proofs();
    dynamics(1000.0,60.0,2000.0,180.0).into_iter().sum::<f64>()
        + uniform_matrix().into_iter().sum::<f64>()
        + mixed_matrix().into_iter().sum::<f64>()
}
