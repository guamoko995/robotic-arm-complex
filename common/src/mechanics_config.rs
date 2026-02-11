use crate::quantities::{Position, Velocity};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

/// Конфигурация инициализации механической части робота при включении.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, MaxSize)]
pub struct StartupMechanicsConfig {
    /// Позиция манипулятора.
    pub init_position: Position,

    /// Ограничение максимальной скорости перемещения по осям.
    pub max_speed: Velocity,
}
