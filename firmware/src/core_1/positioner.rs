use crate::{
    connectors::{PosAckSender, PosReceiver},
    core_1::positioner::{mechanics::servo_motor, utils::SecondsExt as _},
};
use common::{
    quantities::{Position, Velocity},
    units::{Radians, RadiansPerSecond, Seconds},
};
use core::f32::consts::PI;
use embassy_time::Instant;
use esp_hal::{gpio::interconnect::PeripheralOutput, ledc::channel::Error, peripherals::LEDC};
use mechanics::Mechanics;

pub mod mechanics;
pub mod utils;

/// Позиция манипулятора при включении (TODO: перенести в постоянное
/// хранилище).
static INIT_POSITION: Position = Position {
    rotation: Radians::new(1.57),
    shoulder: Radians::new(1.3),
    forearm: Radians::new(0.7),
    claw: Radians::new(2.5),
};

/// Максимальные скорости осей (TODO: перенести в постоянное хранилище).
const MAX_SPEED: Velocity = Velocity {
    rotation: RadiansPerSecond::new(PI / 3.0),
    shoulder: RadiansPerSecond::new(PI / 2.0),
    forearm: RadiansPerSecond::new(PI / 2.0),
    claw: RadiansPerSecond::new(PI),
};

/// Интервал обновления позиции, синхронизированный с частотой ШИМ, управляющим
/// сервомоторами.
const POSITIONING_INTERVAL: Seconds = Seconds::new(1.0 / servo_motor::PWM_FREQ_HZ as f32);

pub struct Positioner(Mechanics);

impl Positioner {
    pub fn make<R, S, F, C>(
        ledc: LEDC<'static>,
        rotation_pin: R,
        shoulder_pin: S,
        forearm_pin: F,
        claw_pin: C,
    ) -> Result<Self, Error>
    where
        R: PeripheralOutput<'static>,
        S: PeripheralOutput<'static>,
        F: PeripheralOutput<'static>,
        C: PeripheralOutput<'static>,
    {
        Ok(Self(Mechanics::make(
            ledc,
            rotation_pin,
            shoulder_pin,
            forearm_pin,
            claw_pin,
        )?))
    }

    /// Задача управления траекторией движения манипулятора.
    ///
    /// Получает целевые позиции, разбивает их на мелкие шаги и плавно перемещает
    /// приводы, соблюдая временные интервалы.
    pub fn run(&'static mut self, pos_rx: PosReceiver, pos_ack_tx: PosAckSender) {
        let mut next_tick = Instant::now();
        let mut current_pos = INIT_POSITION;
        let interval = POSITIONING_INTERVAL.as_duration();

        loop {
            let dst = utils::blocking_receive(&pos_rx);

            for pos in utils::interpolation(current_pos, dst, MAX_SPEED) {
                next_tick += interval;
                // Защита от накопления задержек: если мы отстали, выравниваем время.
                next_tick = next_tick.max(Instant::now());

                while Instant::now() < next_tick {
                    core::hint::spin_loop();
                }

                self.0.set_pos(pos);
                current_pos = pos;
            }
            utils::blocking_ack(&pos_ack_tx);
        }
    }
}
