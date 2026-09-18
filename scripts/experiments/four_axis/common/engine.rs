use core::marker::PhantomData;
use core::ops::{Add,Sub,Mul};
use nalgebra::SMatrix;

// All four prime/angle scale exponents remain independent in both variants.
pub type One=U<0,0,0,0,0,0,0,0>;
pub type Meter=U<0,0,0,0,0,1,0,0>;
pub type Millimeter=U<-3,0,-3,0,0,1,0,0>;
pub type Second=U<0,0,0,0,0,0,1,0>;
pub type Millisecond=U<-3,0,-3,0,0,0,1,0>;
pub type Tick=U<-2,-1,-1,0,0,0,1,0>;
pub type Kilogram=U<0,0,0,0,1,0,0,0>;
pub type Gram=U<-3,0,-3,0,1,0,0,0>;
pub type Radian=U<0,0,0,0,0,0,0,1>;
pub type Degree=U<-2,-2,-1,1,0,0,0,1>;
pub type Velocity=DivUnit<Meter,Second>;
pub type Acceleration=DivUnit<Velocity,Second>;
pub type Force=MulUnit<Kilogram,Acceleration>;
pub type AngularVelocity=DivUnit<Radian,Second>;

#[repr(transparent)]
pub struct Quantity<U,T=f64,Brand=()> {pub raw:T, marker:PhantomData<fn()->(U,Brand)>}
impl<U,T,Brand> Quantity<U,T,Brand> {
    pub const fn new(raw:T)->Self {Self{raw,marker:PhantomData}}
}
impl<U,T:Copy,Brand> Copy for Quantity<U,T,Brand> {}
impl<U,T:Clone,Brand> Clone for Quantity<U,T,Brand> {
    fn clone(&self)->Self {Self::new(self.raw.clone())}
}
impl<U,T:Add<Output=T>,Brand> Add for Quantity<U,T,Brand> {
    type Output=Self;
    fn add(self,rhs:Self)->Self {Self::new(self.raw+rhs.raw)}
}
impl<U,T:Sub<Output=T>,Brand> Sub for Quantity<U,T,Brand> {
    type Output=Self;
    fn sub(self,rhs:Self)->Self {Self::new(self.raw-rhs.raw)}
}

pub trait UnitRescale<Target> {const FACTOR:f64;}
// Sharing D is an exact type equality requirement: scales may differ, dimensions
// cannot. Keeping the same generic impl in both isolates axis representation.
impl<const A2:i16,const A3:i16,const A5:i16,const AP:i16,
     const B2:i16,const B3:i16,const B5:i16,const BP:i16,D>
UnitRescale<Unit<Scale<_2<B2>,_3<B3>,_5<B5>,_Pi<BP>>,D>>
for Unit<Scale<_2<A2>,_3<A3>,_5<A5>,_Pi<AP>>,D> {
    const FACTOR:f64 = float_tables::pow2_float((A2-B2) as i32)
        * float_tables::pow3_float((A3-B3) as i32)
        * float_tables::pow5_float((A5-B5) as i32)
        * float_tables::pow_pi_float((AP-BP) as i32);
}
impl<U,Brand> Quantity<U,f64,Brand> {
    pub fn rescale<V>(self)->Quantity<V,f64,Brand> where U:UnitRescale<V> {
        Quantity::new(self.raw*U::FACTOR)
    }
    pub fn sqrt(self)->Quantity<SqrtUnit<U>,f64,Brand> where U:UnitSqrt {
        Quantity::new(self.raw.sqrt())
    }
}

// The storage is real nalgebra; unit multiplication uses the same associated
// projection as whippyalgebra's uniform matrix multiplication.
pub struct Uniform<U,const R:usize,const C:usize> {
    pub raw:SMatrix<f64,R,C>, marker:PhantomData<U>,
}
impl<U,const R:usize,const C:usize> Uniform<U,R,C> {
    pub fn new(raw:SMatrix<f64,R,C>)->Self {Self{raw,marker:PhantomData}}
}
impl<U,V,const R:usize,const C:usize,const K:usize> Mul<Uniform<V,C,K>> for Uniform<U,R,C>
where U:UnitMul<V> {
    type Output=Uniform<MulUnit<U,V>,R,K>;
    fn mul(self,rhs:Uniform<V,C,K>)->Self::Output {Uniform::new(self.raw*rhs.raw)}
}
impl<U> Uniform<U,3,3> {
    pub fn determinant(&self)->Quantity<index::PowUnit<U,typenum::U3>>
    where U:index::UnitPow<typenum::U3> {
        Quantity::new(self.raw.determinant())
    }
}

// A two-axis mixed state operator. Lists are actual current DCons/ElemAt and
// multiplication requires the identical inner list, as in whippyalgebra.
pub struct Mixed<Rows,Cols,const R:usize,const C:usize> {
    pub raw:SMatrix<f64,R,C>, marker:PhantomData<(Rows,Cols)>,
}
impl<Rows: dims::DimList,Cols:dims::DimList,const R:usize,const C:usize> Mixed<Rows,Cols,R,C> {
    pub fn new(raw:SMatrix<f64,R,C>)->Self {
        assert_eq!(Rows::LEN,R); assert_eq!(Cols::LEN,C);
        Self{raw,marker:PhantomData}
    }
    pub fn entry<const I:usize,const J:usize>(&self)->Quantity<DivUnit<
        <Rows as index::ElemAt<index::Nat<I>>>::Elem,
        <Cols as index::ElemAt<index::Nat<J>>>::Elem>>
    where nalgebra::Const<I>:index::ShapeIndex, nalgebra::Const<J>:index::ShapeIndex,
        Rows:index::ElemAt<index::Nat<I>>, Cols:index::ElemAt<index::Nat<J>>,
        <Rows as index::ElemAt<index::Nat<I>>>::Elem:
            UnitDiv<<Cols as index::ElemAt<index::Nat<J>>>::Elem> {
        Quantity::new(self.raw[(I,J)])
    }
}
impl<Rows: dims::DimList,Inner:dims::DimList,Cols:dims::DimList,
     const R:usize,const C:usize,const K:usize>
Mul<Mixed<Inner,Cols,C,K>> for Mixed<Rows,Inner,R,C> {
    type Output=Mixed<Rows,Cols,R,K>;
    fn mul(self,rhs:Mixed<Inner,Cols,C,K>)->Self::Output {Mixed::new(self.raw*rhs.raw)}
}
