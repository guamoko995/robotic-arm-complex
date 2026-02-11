use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};

use crate::mk_static;

// Сигнал смены статуса транспорта, с указанием используемого wifi интерфейса.
pub type ActiveWifiInterface = Signal<NoopRawMutex, WifiInterface>;
// Сигнал смены целевого режима работы wifi.
pub type TargetConfig = Signal<NoopRawMutex, esp_radio::wifi::ModeConfig>;

/// Источник текущего активного TCP-управления.
/// Позволяет "Мозгам" понять, через какой интерфейс пришел клиент.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WifiInterface {
    /// Управляющее соединение отсутствует.
    None,
    /// Клиент подключен через внешнюю инфраструктуру (домашний роутер).
    Station,
    /// Клиент подключен напрямую к собственной точке доступа робота.
    AccessPoint,
}

pub struct Connectors {
    pub active_wifi_interface: ActiveWifiInterface,
    pub target_config: TargetConfig,
}

impl Connectors {
    pub fn new() -> &'static Self {
        mk_static!(
            Connectors,
            Self {
                active_wifi_interface: ActiveWifiInterface::new(),
                target_config: TargetConfig::new(),
            }
        )
    }
}
