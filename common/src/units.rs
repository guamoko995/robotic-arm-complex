//! # Units of Measurement (Lite)
//!
//! Система физических величин с нулевой стоимостью (zero-cost types), обеспечивающая
//! проверку размерностей на этапе компиляции. Использование `PhantomData` гарантирует,
//! что в рантайме все вычисления будут эквивалентны операциям над `f32`.

use core::{
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};
use libm;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

// --- Базовые единицы (Константы) ---

pub const MILLIMETER: Millimeters = Millimeters::new(1.0);
pub const RADIAN: Radians = Radians::new(1.0);
pub const SECOND: Seconds = Seconds::new(1.0);
pub const SQUARE_MILLIMETER: SquareMillimeters = SquareMillimeters::new(1.0);
pub const RADIAN_PER_SECOND: RadiansPerSecond = RadiansPerSecond::new(1.0);

pub trait Max {
    /// Возвращает максимальное из двух чисел.
    fn max(self, other: Self) -> Self;
}
pub trait Abs {
    /// Возвращает модуль числа.
    fn abs(self) -> Self;
}

/// Обобщенный контейнер для значения с меткой единицы измерения.
#[derive(Debug, Copy, Clone, MaxSize, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct WithUnit<Unit>(f32, PhantomData<Unit>);

impl<Unit> WithUnit<Unit> {
    /// Создает новое значение заданной единицы измерения.
    #[inline]
    pub const fn new(val: f32) -> Self {
        Self(val, PhantomData)
    }
}

impl<Unit> Max for WithUnit<Unit> {
    /// Возвращает максимальное из двух чисел.
    #[inline]
    fn max(self, other: Self) -> Self {
        Self::new(libm::fmaxf(self.0, other.0))
    }
}

impl<Unit> Abs for WithUnit<Unit> {
    /// Возвращает модуль числа.
    #[inline]
    fn abs(self) -> Self {
        Self::new(libm::fabsf(self.0))
    }
}

// --- Конвертация ---

impl<Unit> From<f32> for WithUnit<Unit> {
    #[inline]
    fn from(value: f32) -> Self {
        Self(value, PhantomData)
    }
}

impl<Unit> From<WithUnit<Unit>> for f32 {
    #[inline]
    fn from(value: WithUnit<Unit>) -> Self {
        value.0
    }
}

// --- Арифметика однородных величин ---

impl<Unit> AddAssign for WithUnit<Unit> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl<Unit> Add for WithUnit<Unit> {
    type Output = Self;
    #[inline]
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl<Unit> SubAssign for WithUnit<Unit> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl<Unit> Sub for WithUnit<Unit> {
    type Output = Self;
    #[inline]
    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

// --- Операции со скалярами (f32) ---

impl<Unit> MulAssign<f32> for WithUnit<Unit> {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

impl<Unit> Mul<f32> for WithUnit<Unit> {
    type Output = Self;
    #[inline]
    fn mul(mut self, rhs: f32) -> Self {
        self *= rhs;
        self
    }
}

impl<Unit> Mul<WithUnit<Unit>> for f32 {
    type Output = WithUnit<Unit>;
    #[inline]
    fn mul(self, rhs: WithUnit<Unit>) -> WithUnit<Unit> {
        rhs * self
    }
}

impl<Unit> DivAssign<f32> for WithUnit<Unit> {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.0 /= rhs;
    }
}

impl<Unit> Div<f32> for WithUnit<Unit> {
    type Output = Self;
    #[inline]
    fn div(mut self, rhs: f32) -> Self {
        self /= rhs;
        self
    }
}

/// Деление однородных величин возвращает безразмерный коэффициент.
impl<Unit> Div for WithUnit<Unit> {
    type Output = f32;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

// --- Определения конкретных физических величин ---

#[derive(Debug, Copy, Clone, MaxSize, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Radian;
/// Углы в радианах.
pub type Radians = WithUnit<Radian>;

#[derive(Debug, Copy, Clone, MaxSize, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Millimeter;
/// Линейные размеры в миллиметрах.
pub type Millimeters = WithUnit<Millimeter>;

#[derive(Debug, Copy, Clone, MaxSize, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct SquareMillimeter;
/// Площадь в квадратных миллиметрах.
pub type SquareMillimeters = WithUnit<SquareMillimeter>;

#[derive(Debug, Copy, Clone, MaxSize, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Second;
/// Время в секундах.
pub type Seconds = WithUnit<Second>;

#[derive(Debug, Copy, Clone, MaxSize, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct RadianPerSecond;
/// Угловая скорость.
pub type RadiansPerSecond = WithUnit<RadianPerSecond>;

// --- Геометрические взаимодействия ---

impl Mul for Millimeters {
    type Output = SquareMillimeters;
    #[inline]
    fn mul(self, rhs: Self) -> SquareMillimeters {
        SquareMillimeters::new(self.0 * rhs.0)
    }
}

impl Div<Millimeters> for SquareMillimeters {
    type Output = Millimeters;
    #[inline]
    fn div(self, rhs: Millimeters) -> Millimeters {
        Millimeters::new(self.0 / rhs.0)
    }
}

// --- Кинематические взаимодействия ---

impl Div<Seconds> for Radians {
    type Output = RadiansPerSecond;
    #[inline]
    fn div(self, rhs: Seconds) -> Self::Output {
        RadiansPerSecond::new(self.0 / rhs.0)
    }
}

impl Div<RadiansPerSecond> for Radians {
    type Output = Seconds;
    #[inline]
    fn div(self, rhs: RadiansPerSecond) -> Self::Output {
        Seconds::new(self.0 / rhs.0)
    }
}

impl Mul<Seconds> for RadiansPerSecond {
    type Output = Radians;
    #[inline]
    fn mul(self, rhs: Seconds) -> Self::Output {
        Radians::new(self.0 * rhs.0)
    }
}

impl Mul<RadiansPerSecond> for Seconds {
    type Output = Radians;
    #[inline]
    fn mul(self, rhs: RadiansPerSecond) -> Self::Output {
        Radians::new(self.0 * rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn test_conversions() {
        let val = 42.0;
        let m = Millimeters::new(val);

        assert_eq!(f32::from(m), val);

        let m_from: Millimeters = val.into();
        assert_eq!(m, m_from);
    }

    #[test]
    fn test_addition_logic() {
        let a = Millimeters::new(10.5);
        let b = Millimeters::new(4.5);

        assert_eq!(a + b, Millimeters::new(15.0));

        let mut res = a;
        res += b;
        assert_eq!(res, Millimeters::new(15.0));
    }

    #[test]
    fn test_subtraction_logic() {
        let a = Radians::new(PI);
        let b = Radians::new(PI / 4.0);

        assert_eq!(a - b, Radians::new(3.0 * PI / 4.0));

        let mut res = a;
        res -= b;
        assert_eq!(res, Radians::new(3.0 * PI / 4.0));
    }

    #[test]
    fn test_scalar_multiplication() {
        let m = Millimeters::new(12.0);
        let factor = 2.5;

        assert_eq!(m * factor, Millimeters::new(30.0));
        assert_eq!(factor * m, Millimeters::new(30.0));

        let mut res = m;
        res *= factor;
        assert_eq!(res, Millimeters::new(30.0));
    }

    #[test]
    fn test_scalar_division() {
        let m = Millimeters::new(100.0);
        let divisor = 4.0;

        assert_eq!(m / divisor, Millimeters::new(25.0));

        let mut res = m;
        res /= divisor;
        assert_eq!(res, Millimeters::new(25.0));
    }

    #[test]
    fn test_dimensionless_division() {
        let a = Millimeters::new(50.0);
        let b = Millimeters::new(2.0);
        let ratio: f32 = a / b;
        assert_eq!(ratio, 25.0);
    }

    #[test]
    fn test_complex_geometry() {
        let w = Millimeters::new(10.0);
        let h = Millimeters::new(20.0);

        let area = w * h;
        assert_eq!(area, SquareMillimeters::new(200.0));

        assert_eq!(area / w, h);
    }

    #[test]
    fn test_kinematics() {
        let distance = Radians::new(PI);
        let time = Seconds::new(4.0);
        let expected_speed = RadiansPerSecond::new(PI / 4.0);

        assert_eq!(distance / time, expected_speed);
        assert_eq!(expected_speed * time, distance);
        assert_eq!(time * expected_speed, distance);
    }
}
