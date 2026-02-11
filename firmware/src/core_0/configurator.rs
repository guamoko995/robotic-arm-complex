mod conf_stor;

use crate::{
    core_0::{
        configurator::conf_stor::StorageError,
        connectors::{CmdAckSender, CmdReceiver, SignalConfigUpdated},
    },
    mk_static,
};
use common::request::Command;
use conf_stor::{ConfigStorage, flash_async::Flash};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_hal::peripherals::FLASH;
use esp_println::println;

pub struct Configurator {
    flash: Flash<NoopRawMutex>,
}
impl Configurator {
    pub fn make(flash: FLASH<'static>) -> &'static mut Self {
        mk_static!(
            Configurator,
            Self {
                flash: Flash::new(flash),
            }
        )
    }
    pub async fn run(
        &'static mut self,
        cmd_rx: CmdReceiver<'_>,
        cmd_ack_tx: CmdAckSender<'_>,
        config_updated: &SignalConfigUpdated,
    ) -> ! {
        let Self { flash } = self;
        let mut storage = ConfigStorage::new(flash);

        let initial_cfg = match storage.fetch_wifi().await {
            Ok(cfg) => {
                println!("CONFIGURATOR: config fetched");
                cfg
            }
            Err(StorageError::NotFound) => {
                println!("CONFIGURATOR: config not found");
                println!("CONFIGURATOR: using default config");
                common::wifi_config::WifiConfig::default()
            }
            Err(err) => {
                println!("CONFIGURATOR ERROR: failed to fetched config: {err:?}");
                println!("CONFIGURATOR: using default config");
                common::wifi_config::WifiConfig::default()
            }
        };
        config_updated.signal(initial_cfg);
        loop {
            let command = cmd_rx.receive().await;

            match command {
                Command::ConfigureWifi(new_cfg) => {
                    println!("CONFIGURATOR: saving new WiFi config...");
                    if let Err(e) = storage.store_wifi(new_cfg.clone()).await {
                        println!(
                            "CONFIGURATOR ERROR: failed to save to flash memory: {:?}",
                            e
                        );
                    }
                    config_updated.signal(new_cfg);
                }
                Command::SetMaxSpeed(_) => todo!(),
            }
            cmd_ack_tx.send(()).await
        }
    }
}
