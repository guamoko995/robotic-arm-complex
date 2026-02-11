mod configurator;
mod connectors;
mod network;

use crate::{
    connectors::{PosAckReceiver, PosSender},
    core_0::network::Network,
    mk_static,
};
use configurator::Configurator;
use connectors::Connectors;
use embassy_futures::select::{Either, select};
use esp_hal::{
    peripherals::{FLASH, TIMG0, WIFI},
    timer::timg::TimerGroup,
};

const HEAP_SIZE: usize = 98767;

pub struct Core0 {
    timg0: TIMG0<'static>,
    flash: FLASH<'static>,
    wifi: WIFI<'static>,
}

impl Core0 {
    pub fn make(timg0: TIMG0<'static>, flash: FLASH<'static>, wifi: WIFI<'static>) -> Self {
        esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: HEAP_SIZE);

        Self { timg0, flash, wifi }
    }

    pub async fn run(self, pos_tx: PosSender, pos_ack_rx: PosAckReceiver) -> ! {
        let Self { timg0, flash, wifi } = self;

        let Connectors {
            cmd,
            cmd_ack,
            config_updated,
        } = Connectors::new();

        esp_rtos::start(TimerGroup::new(timg0).timer0);

        // Конфигуратор.
        let cmd_rx = cmd.receiver();
        let cmd_ack_tx = cmd_ack.sender();
        let config_updated_tx = &config_updated;
        let configurator = Configurator::make(flash);

        // Сеть.
        let cmd_tx = cmd.sender();
        let cmd_ack_rx = cmd_ack.receiver();
        let config_updated_rx = &config_updated;
        let network = Network::make(wifi);

        // Запуск конфигуратора и сети.
        match select(
            configurator.run(cmd_rx, cmd_ack_tx, config_updated_tx),
            network.run(pos_tx, pos_ack_rx, cmd_tx, cmd_ack_rx, config_updated_rx),
        )
        .await
        {
            Either::First(never) | Either::Second(never) => never,
        }
    }
}
