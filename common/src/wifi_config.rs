use crate::String;
use enumset::{EnumSet, EnumSetType};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

pub const MAX_SSID_LEN: usize = 32;
pub const MAX_PASS_LEN: usize = 64;

/// Режимы работы Wi-Fi контроллера.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, MaxSize)]
pub struct WifiConfig {
    /// Режим клиента (подключение к внешней точке доступа).
    pub client: Option<ClientConfig>,

    /// Режим точки доступа (робот сам создает сеть).
    pub access_point: Option<AccessPointConfig>,
}

impl Default for WifiConfig {
    fn default() -> Self {
        Self {
            client: None,
            access_point: Some(AccessPointConfig::default()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolsSet(pub EnumSet<Protocol>);

impl Default for ProtocolsSet {
    fn default() -> Self {
        let s: EnumSet<Protocol> = EnumSet::only(Protocol::default());
        ProtocolsSet(s)
    }
}

impl MaxSize for ProtocolsSet {
    // Так как вариантов 6, enumset использует u8 в качестве внутреннего представления.
    // Postcard сериализует u8 как 1 байт.
    const POSTCARD_MAX_SIZE: usize = core::mem::size_of::<u8>();
}

/// Настройки Wi-Fi в режиме клиента (Station).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, MaxSize)]
pub struct ClientConfig {
    /// Имя Wi-Fi сети (SSID).
    pub ssid: String<MAX_SSID_LEN>,

    /// BSSID (MAC-адрес конкретной точки доступа). Если None — подключается к любому SSID с этим именем.
    pub bssid: Option<[u8; 6]>,

    /// Метод аутентификации (безопасности) сети.
    pub auth_method: AuthMethod,

    /// Пароль для подключения к сети.
    pub password: String<MAX_PASS_LEN>,

    /// Номер канала (если известен). Ускоряет процесс подключения.
    pub channel: Option<u8>,

    /// Набор поддерживаемых протоколов (802.11 b/g/n/ax).
    pub protocols: ProtocolsSet,
}

/// Поддерживаемые методы аутентификации Wi-Fi.
#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Deserialize, Serialize, MaxSize)]
pub enum AuthMethod {
    /// Открытая сеть (без пароля).
    None,

    /// Устаревший протокол WEP.
    Wep,

    /// Протокол WPA.
    Wpa,

    /// Протокол WPA2 Personal (наиболее распространенный).
    #[default]
    Wpa2Personal,

    /// Смешанный режим WPA/WPA2 Personal.
    WpaWpa2Personal,

    /// Корпоративный режим WPA2 Enterprise (с RADIUS-сервером).
    Wpa2Enterprise,

    /// Современный протокол WPA3 Personal.
    Wpa3Personal,

    /// Смешанный режим WPA2/WPA3 Personal.
    Wpa2Wpa3Personal,

    /// Китайский стандарт безопасности WAPI.
    WapiPersonal,
}

/// Поддерживаемые сетевые протоколы Wi-Fi.
#[derive(Debug, Default, PartialOrd, EnumSetType, Deserialize, Serialize, MaxSize)]
pub enum Protocol {
    /// Протокол 802.11b (до 11 Мбит/с).
    P802D11B,

    /// Смешанный режим 802.11b/g (до 54 Мбит/с).
    P802D11BG,

    /// Стандартный режим 802.11b/g/n (до 150 Мбит/с).
    #[default]
    P802D11BGN,

    /// Режим 802.11b/g/n с поддержкой Long-Range (увеличенная дальность).
    P802D11BGNLR,

    /// Специальный протокол 802.11 Long-Range (только для устройств ESP).
    P802D11LR,

    /// Современный стандарт 802.11ax (Wi-Fi 6).
    P802D11BGNAX,
}

/// Конфигурация точки доступа (SoftAP), создаваемой устройством.
#[derive(Clone, PartialEq, Deserialize, Serialize, MaxSize)]
pub struct AccessPointConfig {
    /// Имя создаваемой сети (SSID).
    pub ssid: String<MAX_SSID_LEN>,

    /// Если true — сеть будет скрытой (не видна при сканировании).
    pub ssid_hidden: bool,

    /// Основной радиоканал (1-13).
    pub channel: u8,

    /// Протоколы, которые будет поддерживать точка доступа.
    pub protocols: ProtocolsSet,

    /// Метод аутентификации для подключающихся клиентов.
    pub auth_method: AuthMethod,

    /// Пароль точки доступа (минимум 8 символов для WPA).
    pub password: String<MAX_PASS_LEN>,
}

impl Default for AccessPointConfig {
    fn default() -> Self {
        Self {
            ssid: <String<MAX_SSID_LEN>>::from(
                <heapless::String<MAX_SSID_LEN>>::try_from("robo-arm").unwrap(),
            ),
            ssid_hidden: false,
            channel: 1,
            protocols: ProtocolsSet(
                Protocol::P802D11B | Protocol::P802D11BG | Protocol::P802D11BGN,
            ),
            auth_method: AuthMethod::None,
            password: <String<MAX_PASS_LEN>>::new(),
        }
    }
}

impl core::fmt::Debug for AccessPointConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AccessPointConfig")
            .field("ssid", &self.ssid)
            .field("ssid_hidden", &self.ssid_hidden)
            .field("channel", &self.channel)
            .field("protocols", &self.protocols)
            .field("auth_method", &self.auth_method)
            // Скрываем пароль в логах для безопасности
            .field("password", &"**HIDDEN**")
            .finish()
    }
}
