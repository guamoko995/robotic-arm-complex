use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::units::{Abs, Max, Radians, RadiansPerSecond, Seconds, WithUnit};

pub trait MaxAbsComponent {
    type Output;
    /// Возвращает максимальное из двух чисел.
    fn max_abs_component(self) -> Self::Output;
}

impl<Unit> MaxAbsComponent for Quantity<Unit>
where
    Unit: Max + Abs,
{
    type Output = Unit;
    /// Возвращает максимальный модуль компонента.
    #[inline]
    fn max_abs_component(self) -> Self::Output {
        self.rotation
            .abs()
            .max(self.shoulder.abs())
            .max(self.forearm.abs())
            .max(self.claw.abs())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, MaxSize)]
pub struct Quantity<Unit> {
    pub rotation: Unit,
    pub shoulder: Unit,
    pub forearm: Unit,
    pub claw: Unit,
}

impl<Unit> AddAssign for Quantity<Unit>
where
    Unit: AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        self.rotation += rhs.rotation;
        self.shoulder += rhs.shoulder;
        self.forearm += rhs.forearm;
        self.claw += rhs.claw;
    }
}

impl<Unit> Add for Quantity<Unit>
where
    Quantity<Unit>: AddAssign,
{
    type Output = Quantity<Unit>;
    fn add(self, rhs: Self) -> Self::Output {
        let mut s = self;
        s += rhs;
        s
    }
}

impl<Unit> SubAssign for Quantity<Unit>
where
    Unit: SubAssign,
{
    fn sub_assign(&mut self, rhs: Self) {
        self.rotation -= rhs.rotation;
        self.shoulder -= rhs.shoulder;
        self.forearm -= rhs.forearm;
        self.claw -= rhs.claw;
    }
}

impl<Unit> Sub for Quantity<Unit>
where
    Quantity<Unit>: SubAssign,
{
    type Output = Quantity<Unit>;
    fn sub(self, rhs: Self) -> Self::Output {
        let mut s = self;
        s -= rhs;
        s
    }
}

impl<Unit, T> MulAssign<T> for Quantity<Unit>
where
    Unit: MulAssign<T>,
    T: Copy,
{
    fn mul_assign(&mut self, rhs: T) {
        self.rotation *= rhs;
        self.shoulder *= rhs;
        self.forearm *= rhs;
        self.claw *= rhs;
    }
}

impl<U1, U2> Mul<WithUnit<U2>> for Quantity<WithUnit<U1>>
where
    WithUnit<U1>: Mul<WithUnit<U2>>,
    WithUnit<U2>: Copy,
{
    type Output = Quantity<<WithUnit<U1> as Mul<WithUnit<U2>>>::Output>;
    fn mul(self, rhs: WithUnit<U2>) -> Self::Output {
        Self::Output {
            rotation: self.rotation * rhs,
            shoulder: self.shoulder * rhs,
            forearm: self.forearm * rhs,
            claw: self.claw * rhs,
        }
    }
}

impl<U1> Mul<f32> for Quantity<WithUnit<U1>>
where
    WithUnit<U1>: Mul<f32>,
{
    type Output = Quantity<<WithUnit<U1> as Mul<f32>>::Output>;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::Output {
            rotation: self.rotation * rhs,
            shoulder: self.shoulder * rhs,
            forearm: self.forearm * rhs,
            claw: self.claw * rhs,
        }
    }
}

impl<Unit1, Unit2> Mul<Quantity<Unit2>> for Quantity<Unit1>
where
    Unit1: Mul<Unit2>,
{
    type Output = Quantity<<Unit1 as Mul<Unit2>>::Output>;
    fn mul(self, rhs: Quantity<Unit2>) -> Self::Output {
        Self::Output {
            rotation: self.rotation * rhs.rotation,
            shoulder: self.shoulder * rhs.shoulder,
            forearm: self.forearm * rhs.forearm,
            claw: self.claw * rhs.claw,
        }
    }
}

impl<Unit> Mul<Quantity<Unit>> for f32
where
    Quantity<Unit>: Mul<f32>,
{
    type Output = <Quantity<Unit> as Mul<f32>>::Output;
    fn mul(self, rhs: Quantity<Unit>) -> Self::Output {
        rhs * self
    }
}

impl<Unit, T> Mul<Quantity<Unit>> for WithUnit<T>
where
    Quantity<Unit>: Mul<WithUnit<T>>,
{
    type Output = <Quantity<Unit> as Mul<WithUnit<T>>>::Output;
    fn mul(self, rhs: Quantity<Unit>) -> Self::Output {
        rhs * self
    }
}

impl<Unit, T> DivAssign<T> for Quantity<Unit>
where
    Unit: DivAssign<T>,
    T: Copy,
{
    fn div_assign(&mut self, rhs: T) {
        self.rotation /= rhs;
        self.shoulder /= rhs;
        self.forearm /= rhs;
        self.claw /= rhs;
    }
}

impl<U1, U2> Div<WithUnit<U2>> for Quantity<WithUnit<U1>>
where
    WithUnit<U1>: Div<WithUnit<U2>>,
    WithUnit<U2>: Copy,
{
    type Output = Quantity<<WithUnit<U1> as Div<WithUnit<U2>>>::Output>;
    fn div(self, rhs: WithUnit<U2>) -> Self::Output {
        Self::Output {
            rotation: self.rotation / rhs,
            shoulder: self.shoulder / rhs,
            forearm: self.forearm / rhs,
            claw: self.claw / rhs,
        }
    }
}

impl<Unit> Div<f32> for Quantity<Unit>
where
    Unit: Div<f32>,
{
    type Output = Quantity<<Unit as Div<f32>>::Output>;
    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            rotation: self.rotation / rhs,
            shoulder: self.shoulder / rhs,
            forearm: self.forearm / rhs,
            claw: self.claw / rhs,
        }
    }
}

impl<Unit1, Unit2> Div<Quantity<Unit2>> for Quantity<Unit1>
where
    Unit1: Div<Unit2>,
{
    type Output = Quantity<<Unit1 as Div<Unit2>>::Output>;
    fn div(self, rhs: Quantity<Unit2>) -> Self::Output {
        Self::Output {
            rotation: self.rotation / rhs.rotation,
            shoulder: self.shoulder / rhs.shoulder,
            forearm: self.forearm / rhs.forearm,
            claw: self.claw / rhs.claw,
        }
    }
}

pub type Position = Quantity<Radians>;
pub type Velocity = Quantity<RadiansPerSecond>;
pub type Duration = Quantity<Seconds>;
