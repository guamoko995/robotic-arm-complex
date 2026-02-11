#![no_std]
pub mod mechanics_config;
pub mod quantities;
pub mod request;
pub mod response;
pub mod units;
pub mod wifi_config;

use heapless::{self, CapacityError};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

pub use postcard::{from_bytes, to_vec};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]

pub struct String<const N: usize>(heapless::String<N>);

impl<const N: usize> String<N> {
    #[inline]
    pub fn new() -> Self {
        Self(<heapless::String<N>>::new())
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<const N: usize> MaxSize for String<N> {
    const POSTCARD_MAX_SIZE: usize = N + varint_size(N);
}

impl<const N: usize> From<heapless::String<N>> for String<N> {
    #[inline]
    fn from(value: heapless::String<N>) -> Self {
        Self(value)
    }
}

impl<'a, const N: usize> TryFrom<&'a str> for String<N> {
    type Error = CapacityError;
    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match <heapless::String<N>>::try_from(s) {
            Ok(res) => Ok(Self(res)),
            Err(err) => Err(err),
        }
    }
}

/// Вычисляет количество байт, необходимых для кодирования длины n в формате LEB128 (varint).
/// Применимо для длин строк и векторов в postcard.
#[inline]
pub const fn varint_size(n: usize) -> usize {
    if n < (1 << 7) {
        1
    } else if n < (1 << 14) {
        2
    } else if n < (1 << 21) {
        3
    } else if n < (1 << 28) {
        4
    } else {
        5
    }
}
