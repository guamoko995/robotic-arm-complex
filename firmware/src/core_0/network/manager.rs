use crate::core_0::network::{
    SignalConfigUpdated,
    connectors::{ActiveWifiInterface, TargetConfig, WifiInterface},
};
use common::wifi_config::{AuthMethod, Protocol, WifiConfig};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use enumset::EnumSet;
use esp_println::println;
use esp_radio::wifi::{
    AccessPointConfig, AuthMethod as EspAuth, ClientConfig, ModeConfig, Protocol as EspProtocol,
};

/// Таймаут ожидания TCP-клиента до перехода в режим AP+STA.
const SURVIVAL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq)]
enum ManagerState {
    /// Попытка работы в чистом режиме (STA или AP).
    Optimistic,
    /// TCP-клиент подключен, режим зафиксирован.
    Locked,
    /// Клиент не пришел, расширяем эфир до AP+STA.
    Survival,
}

pub struct Manager(());

impl Manager {
    pub fn make() -> Self {
        Self(())
    }

    pub async fn run(
        &self,
        config_updated_rx: &SignalConfigUpdated,
        used_wifi_interface: &ActiveWifiInterface,
        target_config: &TargetConfig,
    ) -> ! {
        // Шаг 1: Ждем инициализирующий конфиг от бизнеса (прочитанный из Flash при старте)
        let mut config = config_updated_rx.wait().await;
        let mut state = ManagerState::Optimistic;

        loop {
            // Определяем, что отправить в Radio-слой
            let mode_to_send = match state {
                ManagerState::Optimistic | ManagerState::Locked => config.to_pure_config(),
                ManagerState::Survival => config.clone().to_survival_config(),
            };

            // Командуем Провайдеру
            target_config.signal(mode_to_send);

            // Внутренний цикл управления состоянием
            match state {
                ManagerState::Optimistic => {
                    // Ждем либо TCP-клиента, либо обновления конфига, либо истечения таймера
                    match select(
                        select(used_wifi_interface.wait(), config_updated_rx.wait()),
                        Timer::after(SURVIVAL_TIMEOUT),
                    )
                    .await
                    {
                        // Событие от транспорта или новый конфиг
                        Either::First(Either::First(interface)) => {
                            if interface != WifiInterface::None {
                                println!(
                                    "MANAGER: Client connected via {:?}. Locking mode.",
                                    interface
                                );
                                state = ManagerState::Locked;
                            }
                        }
                        Either::First(Either::Second(new_cfg)) => {
                            println!("MANAGER: Config updated, resetting logic.");
                            config = new_cfg;
                            state = ManagerState::Optimistic;
                        }
                        // Таймер истек
                        Either::Second(_) => {
                            if config.client.is_some() {
                                println!("MANAGER: Survival timeout! Enabling AP+STA.");
                                state = ManagerState::Survival;
                            } else {
                                // Если мы и так в AP (нет STA в конфиге), таймер ничего не меняет
                                state = ManagerState::Locked;
                            }
                        }
                    }
                }

                ManagerState::Locked => {
                    // Ждем либо разрыва связи, либо изменения настроек
                    match select(used_wifi_interface.wait(), config_updated_rx.wait()).await {
                        Either::First(interface) => {
                            if interface == WifiInterface::None {
                                println!(
                                    "MANAGER: No active clients. Returning to Optimistic hunt."
                                );
                                state = ManagerState::Optimistic;
                            }
                        }
                        Either::Second(new_cfg) => {
                            config = new_cfg;
                            state = ManagerState::Optimistic;
                        }
                    }
                }

                ManagerState::Survival => {
                    // В режиме выживания ждем только клиента или новый конфиг
                    match select(used_wifi_interface.wait(), config_updated_rx.wait()).await {
                        Either::First(interface) => {
                            if interface != WifiInterface::None {
                                println!("MANAGER: Found client in Survival mode. Locking.");
                                state = ManagerState::Locked;
                            }
                        }
                        Either::Second(new_cfg) => {
                            config = new_cfg;
                            state = ManagerState::Optimistic;
                        }
                    }
                }
            }
        }
    }
}

trait WifiConfigExt {
    fn to_survival_config(self) -> ModeConfig;
    fn to_pure_config(&self) -> ModeConfig;
    fn get_ap_config(&self) -> AccessPointConfig;
    fn get_sta_config(&self) -> Option<ClientConfig>;
}

impl WifiConfigExt for WifiConfig {
    // Вспомогательный метод для сборки AP части
    fn get_ap_config(&self) -> AccessPointConfig {
        let ap_src = self.access_point.clone().unwrap_or_default();
        AccessPointConfig::default()
            .with_ssid(ap_src.ssid.as_str().into())
            .with_password(ap_src.password.as_str().into())
            .with_channel(ap_src.channel)
            .with_auth_method(ap_src.auth_method.to_esp_auth())
            .with_protocols(ap_src.protocols.0.to_enum_set_esp_protocol())
    }

    // Вспомогательный метод для сборки Client части
    fn get_sta_config(&self) -> Option<ClientConfig> {
        self.client.as_ref().map(|c| {
            let mut client = ClientConfig::default()
                .with_ssid(c.ssid.as_str().into())
                .with_password(c.password.as_str().into())
                .with_auth_method(c.auth_method.to_esp_auth());
            if let Some(bssid) = c.bssid {
                client = client.with_bssid(bssid);
            }
            client
        })
    }

    // Режим выживания (AP+STA)
    fn to_survival_config(self) -> ModeConfig {
        match self.get_sta_config() {
            Some(sta) => ModeConfig::ApSta(sta, self.get_ap_config()),
            None => ModeConfig::AccessPoint(self.get_ap_config()),
        }
    }

    // Чистый режим (Либо только STA, либо только AP)
    fn to_pure_config(&self) -> ModeConfig {
        match self.get_sta_config() {
            Some(sta) => ModeConfig::Client(sta), // ТУТ ГЛАВНОЕ: Чистый STA без AP
            None => ModeConfig::AccessPoint(self.get_ap_config()),
        }
    }
}

trait AuthMethodExt {
    fn to_esp_auth(&self) -> EspAuth;
}
impl AuthMethodExt for AuthMethod {
    fn to_esp_auth(&self) -> EspAuth {
        match self {
            AuthMethod::None => EspAuth::None,
            AuthMethod::Wep => EspAuth::Wep,
            AuthMethod::Wpa => EspAuth::Wpa,
            AuthMethod::Wpa2Personal => EspAuth::Wpa2Personal,
            AuthMethod::WpaWpa2Personal => EspAuth::WpaWpa2Personal,
            AuthMethod::Wpa2Enterprise => EspAuth::Wpa2Enterprise,
            AuthMethod::Wpa3Personal => EspAuth::Wpa3Personal,
            AuthMethod::Wpa2Wpa3Personal => EspAuth::Wpa2Wpa3Personal,
            AuthMethod::WapiPersonal => EspAuth::WapiPersonal,
        }
    }
}

trait EnumSetProtocolExt {
    fn to_enum_set_esp_protocol(self) -> EnumSet<EspProtocol>;
}
impl EnumSetProtocolExt for EnumSet<Protocol> {
    fn to_enum_set_esp_protocol(self) -> EnumSet<EspProtocol> {
        let mut set = EnumSet::new();
        for p in self {
            match p {
                Protocol::P802D11B => set.insert(EspProtocol::P802D11B),
                Protocol::P802D11BG => set.insert(EspProtocol::P802D11BG),
                Protocol::P802D11BGN => set.insert(EspProtocol::P802D11BGN),
                Protocol::P802D11BGNLR => set.insert(EspProtocol::P802D11BGNLR),
                Protocol::P802D11LR => set.insert(EspProtocol::P802D11LR),
                Protocol::P802D11BGNAX => set.insert(EspProtocol::P802D11BGNAX),
            };
        }
        set
    }
}
