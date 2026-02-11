use crate::connectors::{PosAckSender, PosReceiver};
use common::{
    quantities::{MaxAbsComponent, Position, Velocity},
    units::Seconds,
};
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use embassy_time::Duration;

/// Расширение для перевода физических секунд в длительность Embassy.
pub trait SecondsExt {
    fn as_duration(&self) -> Duration;
}

impl SecondsExt for Seconds {
    fn as_duration(&self) -> Duration {
        // Используем микросекунды для сохранения точности f32.
        Duration::from_micros((f32::from(*self) * 1e6) as u64)
    }
}

/// Итератор для плавного перемещения между двумя точками.
pub struct Interpolator {
    target_pos: Position,
    step: Position,
    steps: u32,
}

impl Iterator for Interpolator {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.steps == 0 {
            return None;
        }

        let m = (self.steps - 1) as f32;
        let pos = self.target_pos - (self.step * m);
        self.steps -= 1;
        Some(pos)
    }
}

/// Рассчитывает шаги интерполяции исходя из максимально допустимых скоростей.
pub fn interpolation(src: Position, dst: Position, max_speed: Velocity) -> Interpolator {
    let delta = dst - src;

    // Вычисляем время движения по самой медленной оси.
    let movement_duration = (delta / max_speed).max_abs_component();
    let steps = libm::ceilf(movement_duration / super::POSITIONING_INTERVAL) as u32;

    let step: Position;
    if steps == 0 {
        step = delta;
    } else {
        step = delta / steps as f32;
    }

    Interpolator {
        target_pos: dst,
        step: step,
        steps: steps,
    }
}

/// Пустой Waker, который ничего не делает.
/// Нужен, так как poll_receive требует Context.
fn noop_waker() -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(core::ptr::null(), &VTABLE), // clone
        |_| {},                                        // wake
        |_| {},                                        // wake_by_ref
        |_| {},                                        // drop
    );
    unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) }
}

pub fn blocking_receive(rx: &PosReceiver) -> Position {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    loop {
        match rx.poll_receive(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => {
                core::hint::spin_loop();
            }
        }
    }
}

pub fn blocking_ack(tx: &PosAckSender) {
    while tx.try_send(()).is_err() {
        core::hint::spin_loop();
    }
}
