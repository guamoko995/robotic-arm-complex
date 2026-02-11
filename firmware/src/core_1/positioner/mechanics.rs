pub mod pwm;
pub mod servo_motor;

use common::quantities::Position;
use esp_hal::{
    gpio::interconnect::PeripheralOutput,
    ledc::channel::{self, Error},
    peripherals::LEDC,
};
use esp_println as _;
use pwm::PWM;
use servo_motor::Servo;

/// Механика робота, объединяющая четыре сервопривода манипулятора.
pub struct Mechanics {
    rotation: Servo,
    shoulder: Servo,
    forearm: Servo,
    claw: Servo,
}

impl Mechanics {
    /// Инициализирует механику робота, назначая каждому узлу свой канал ШИМ и пин.
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
        let pwm = PWM::make(ledc).expect("failed make PWM");
        Ok(Self {
            rotation: Servo::init(pwm, channel::Number::Channel0, rotation_pin)?,
            shoulder: Servo::init(pwm, channel::Number::Channel1, shoulder_pin)?,
            forearm: Servo::init(pwm, channel::Number::Channel2, forearm_pin)?,
            claw: Servo::init(pwm, channel::Number::Channel3, claw_pin)?,
        })
    }

    /// Устанавливает положение всех узлов манипулятора на основе структуры Position.
    pub fn set_pos(&mut self, pos: Position) {
        self.rotation.set_pos(pos.rotation);
        self.shoulder.set_pos(pos.shoulder);
        self.forearm.set_pos(pos.forearm);
        self.claw.set_pos(pos.claw);
    }
}
