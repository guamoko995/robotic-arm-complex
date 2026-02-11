//! Сетевой стек манипулятора.
//!
//! Модуль обеспечивает работу устройства в сети Wi-Fi и предоставляет
//! асинхронный TCP-сервер для приема команд управления и отправки подтверждений.

use crate::{
    connectors::{PosAckReceiver, PosSender},
    core_0::{
        connectors::{CmdAckReceiver, CmdSender, SignalConfigUpdated},
        network::connectors::ActiveWifiInterface,
    },
    mk_static,
};
use connectors::Connectors;
use embassy_futures::select::{Either5, select5};
use embassy_net::{DhcpConfig, Runner, StackResources};
use esp_hal::{peripherals::WIFI, rng::Rng};
use esp_radio::wifi::{self, WifiDevice};
use link::WifiProvider;
use manager::Manager;
use transport::MultiLinkTransport;

mod api;
mod connectors;
mod link;
mod manager;
mod transport;

pub struct Network {
    manager: Manager,
    transport: MultiLinkTransport,
    wifi_provider: WifiProvider<'static>,
    sta_runner: Runner<'static, WifiDevice<'static>>,
    ap_runner: Runner<'static, WifiDevice<'static>>,
}
impl Network {
    pub fn make(wifi: WIFI<'static>) -> &'static mut Self {
        let (controller, interfaces) = wifi::new(link::init_radio(), wifi, Default::default())
            .expect("NETWORK: failed to initialize Wi-Fi controller");

        let wifi_provider = WifiProvider::new(controller);

        let rng = Rng::new();

        let random_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
        let (sta_stack, sta_runner) = embassy_net::new(
            interfaces.sta,
            embassy_net::Config::dhcpv4(DhcpConfig::default()),
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            random_seed,
        );

        let random_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
        let (ap_stack, ap_runner) = embassy_net::new(
            interfaces.ap,
            embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
                address: embassy_net::Ipv4Cidr::new(
                    embassy_net::Ipv4Address::new(192, 168, 4, 1),
                    24,
                ),
                gateway: None,
                dns_servers: Default::default(),
            }),
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            random_seed,
        );

        mk_static!(
            Network,
            Self {
                manager: Manager::make(),
                transport: MultiLinkTransport::make(ap_stack, sta_stack),
                wifi_provider,
                sta_runner,
                ap_runner,
            }
        )
    }

    pub async fn run(
        &'static mut self,
        pos_tx: PosSender,
        pos_ack_rx: PosAckReceiver,
        cmd_tx: CmdSender<'static>,
        cmd_ack_rx: CmdAckReceiver<'static>,
        config_updated_rx: &SignalConfigUpdated,
    ) -> ! {
        let Self {
            manager,
            transport,
            wifi_provider,
            sta_runner,
            ap_runner,
        } = self;
        let Connectors {
            active_wifi_interface,
            target_config,
        } = Connectors::new();

        match select5(
            manager.run(config_updated_rx, &active_wifi_interface, target_config),
            transport.run(
                pos_tx,
                pos_ack_rx,
                cmd_tx,
                cmd_ack_rx,
                &active_wifi_interface,
            ),
            wifi_provider.run(target_config),
            sta_runner.run(),
            ap_runner.run(),
        )
        .await
        {
            Either5::First(v)
            | Either5::Second(v)
            | Either5::Third(v)
            | Either5::Fourth(v)
            | Either5::Fifth(v) => v,
        }
    }
}
