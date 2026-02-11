mod positioner;

use crate::{
    connectors::{PosAckSender, PosReceiver},
    mk_static,
};
use esp_hal::{
    gpio::interconnect::PeripheralOutput,
    peripherals::{CPU_CTRL, LEDC},
    system::{self, AppCoreGuard, CpuControl, Stack},
};
use positioner::Positioner;

const CORE1_STACK_SIZE: usize = 16384;
pub struct Core1<
    R: PeripheralOutput<'static>,
    S: PeripheralOutput<'static>,
    F: PeripheralOutput<'static>,
    C: PeripheralOutput<'static>,
> {
    cpu_control: CpuControl<'static>,
    ledc: LEDC<'static>,
    rotation_pin: R,
    shoulder_pin: S,
    forearm_pin: F,
    claw_pin: C,
}

impl<R, S, F, C> Core1<R, S, F, C>
where
    R: PeripheralOutput<'static> + Send + 'static,
    S: PeripheralOutput<'static> + Send + 'static,
    F: PeripheralOutput<'static> + Send + 'static,
    C: PeripheralOutput<'static> + Send + 'static,
{
    pub fn make(
        cpu_control: CPU_CTRL<'static>,
        ledc: LEDC<'static>,
        rotation_pin: R,
        shoulder_pin: S,
        forearm_pin: F,
        claw_pin: C,
    ) -> Self {
        Self {
            cpu_control: CpuControl::new(cpu_control),
            ledc,
            rotation_pin,
            shoulder_pin,
            forearm_pin,
            claw_pin,
        }
    }

    pub fn run(
        self,
        pos_rx: PosReceiver,
        pos_ack_tx: PosAckSender,
    ) -> Result<AppCoreGuard<'static>, system::Error> {
        let Self {
            mut cpu_control,
            ledc,
            rotation_pin,
            shoulder_pin,
            forearm_pin,
            claw_pin,
        } = self;

        let stack = mk_static!(Stack::<CORE1_STACK_SIZE>, Stack::new());

        cpu_control.start_app_core(stack, move || {
            let positioner =
                Positioner::make(ledc, rotation_pin, shoulder_pin, forearm_pin, claw_pin)
                    .expect("failed to make the positioner");
            let positioner = mk_static!(Positioner, positioner);
            positioner.run(pos_rx, pos_ack_tx);
        })
    }
}
