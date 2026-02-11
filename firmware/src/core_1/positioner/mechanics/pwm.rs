use crate::{core_1::positioner::mechanics::servo_motor, mk_static};
use esp_hal::{
    ledc::{
        HighSpeed, Ledc,
        timer::{self, Timer, TimerIFace},
    },
    peripherals::LEDC,
    time::Rate,
};

/// Частота сигнала ШИМ для управления сервомотором.
const FREQUENCY: Rate = Rate::from_hz(servo_motor::PWM_FREQ_HZ);

/// Контроллер ШИМ для управления группой серводвигателей.
///
/// Обеспечивает общую базу времени (таймер) для всех подключенных каналов серво.
pub struct PWM {
    /// Доступ к периферии LEDC (контроллер ШИМ).
    pub ledc: Ledc<'static>,
    /// Высокоскоростной таймер, настроенный на 50 Гц.
    pub hstimer: Timer<'static, HighSpeed>,
}

impl PWM {
    /// Создает новый экземпляр контроллера и настраивает таймер.
    ///
    /// Использует 14-битную точность (Duty14Bit) для плавного управления углом.
    ///
    /// # Errors
    /// Возвращает `timer::Error`, если конфигурация таймера не поддерживается выбранным источником тактирования.
    pub fn make(ledc: LEDC<'static>) -> Result<&'static mut PWM, timer::Error> {
        let ledc = Ledc::new(ledc);
        let mut hstimer = ledc.timer::<HighSpeed>(timer::Number::Timer0);

        hstimer.configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::HSClockSource::APBClk,
            frequency: FREQUENCY,
        })?;

        Ok(mk_static!(PWM, PWM { ledc, hstimer }))
    }
}
