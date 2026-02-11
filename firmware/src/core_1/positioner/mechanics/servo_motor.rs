use super::PWM;
use common::units::Radians;
use core::f32::consts::PI;
use embedded_hal::pwm::SetDutyCycle;
use esp_hal::{
    gpio::{DriveMode, interconnect::PeripheralOutput},
    ledc::{
        HighSpeed,
        channel::{Channel, ChannelIFace, Error, Number, config::Config},
    },
};

/// Частота управления сервоприводом в герцах.
pub const PWM_FREQ_HZ: u32 = 50;

/// Период ШИМ в микросекундах.
const PERIOD_US: u32 = 1_000_000 / PWM_FREQ_HZ;

/// Минимальная ширина импульса (0.5 мс = 500 мкс).
const MIN_PULSE_US: u32 = 500;

/// Максимальная ширина импульса (2.5 мс = 2500 мкс).
const MAX_PULSE_US: u32 = 2500;

/// Управляемый серводвигатель.
///
/// Работает на базе LEDC ШИМ контроллера ESP32.
/// Рассчитан на стандартные сервоприводы с частотой обновления 50 Гц.
pub struct Servo {
    chan: Channel<'static, HighSpeed>,
    min_duty: u16,
    max_pos: u16,
}

impl Servo {
    /// Инициализирует серводвигатель и привязывает его к каналу ШИМ.
    ///
    /// # Внимание
    /// Перед вызовом убедитесь, что `pwm.hstimer` настроен на частоту **50 Гц** (период 20 мс).
    ///
    /// # Аргументы
    /// * `pwm` - Ссылка на инициализированную периферию PWM.
    /// * `chan_num` - Номер канала LEDC (например, `Number::Channel0`).
    /// * `pin` - Пин, к которому подключен сигнальный провод серво (PWM).
    pub fn init<T: PeripheralOutput<'static>>(
        pwm: &'static PWM,
        chan_num: Number,
        pin: T,
    ) -> Result<Servo, Error> {
        let mut chan = pwm.ledc.channel(chan_num, pin);
        chan.configure(Config {
            timer: &pwm.hstimer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })?;

        let max_duty_cycle = chan.max_duty_cycle() as u32;

        // TODO в константы. Минимальный коэффициент заполнения (2.5% от периода) -> 0.5 мс
        let min_duty = (MIN_PULSE_US * max_duty_cycle / PERIOD_US) as u16;

        //  TODO в константы. Максимальный коэффициент заполнения (12.5% от периода) -> 2.5 мс
        let max_duty = (MAX_PULSE_US * max_duty_cycle / PERIOD_US) as u16;

        let max_pos = max_duty - min_duty;
        Ok(Servo {
            chan,
            min_duty,
            max_pos,
        })
    }

    /// Задаёт позицию качалки серводвигателя.
    ///
    /// # Аргументы
    /// * `pos` - Угол в радианах. Ожидаемый диапазон: [0, PI].
    ///
    /// # Ошибки
    /// Возвращает `Error`, если не удалось обновить коэффициент заполнения ШИМ.
    pub fn set_pos(&mut self, pos: Radians) {
        let mut rad: f32 = pos.into();

        // Ограничиваем угол диапазоном [0, PI] для безопасности
        rad = rad.clamp(0.0, PI);

        let duty = (rad * (self.max_pos) as f32 / PI) as u16 + self.min_duty;

        // Вызываемая функция всегда возвращает Ok(())
        self.chan.set_duty_cycle(duty).unwrap()
    }
}
