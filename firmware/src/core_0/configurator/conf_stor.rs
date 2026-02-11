//! # Configuration Storage
//!
//! Модуль обеспечивает высокоуровневый асинхронный интерфейс для сохранения
//! и извлечения настроек робота во Flash-памяти ESP32.

pub mod flash_async;
mod utils;

use common::{mechanics_config::StartupMechanicsConfig, wifi_config::WifiConfig};
use core::ops::Range;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embedded_storage_async::nor_flash::ErrorType;
use flash_async::Flash;
use postcard::{from_bytes, to_slice};
use sequential_storage::{
    cache::NoCache,
    map::{MapConfig, MapStorage, SerializationError, Value},
};

/// Ошибки при работе с хранилищем конфигурации.
#[derive(Debug)]
pub enum StorageError<E> {
    /// Ошибка на уровне драйвера Flash или структуры данных библиотеки.
    Flash(sequential_storage::Error<E>),
    /// Данные по запрошенному ключу отсутствуют в памяти.
    NotFound,
}

const STORAGE_PARTITION_OFFSET: u32 = 0x2A0000;
const STORAGE_PARTITION_SIZE: u32 = 0x20000;

const STORAGE_PARTITION_RANGE: Range<u32> = Range {
    start: STORAGE_PARTITION_OFFSET,
    end: STORAGE_PARTITION_OFFSET + STORAGE_PARTITION_SIZE,
};

#[repr(u8)]
enum ConfigKey {
    Mechanics = 0,
    Wifi = 1,
}

struct MechanicsEntry(StartupMechanicsConfig);
struct WifiEntry(WifiConfig);

impl<'a> Value<'a> for MechanicsEntry {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        let data = to_slice(&self.0, buffer).map_err(|_| SerializationError::Custom(0))?;
        Ok(data.len())
    }

    fn deserialize_from(buffer: &'a [u8]) -> Result<(Self, usize), SerializationError>
    where
        Self: Sized,
    {
        let item = from_bytes(buffer).map_err(|_| SerializationError::Custom(0))?;
        Ok((MechanicsEntry(item), buffer.len()))
    }
}

impl<'a> Value<'a> for WifiEntry {
    fn serialize_into(&self, buffer: &mut [u8]) -> Result<usize, SerializationError> {
        let data = to_slice(&self.0, buffer).map_err(|_| SerializationError::Custom(0))?;
        Ok(data.len())
    }

    fn deserialize_from(buffer: &'a [u8]) -> Result<(Self, usize), SerializationError>
    where
        Self: Sized,
    {
        let item = from_bytes(buffer).map_err(|_| SerializationError::Custom(0))?;
        Ok((WifiEntry(item), buffer.len()))
    }
}

/// Асинхронный менеджер конфигурации.

pub struct ConfigStorage<'a, M: RawMutex>(MapStorage<u8, &'a Flash<M>, NoCache>);

impl<'a, M: RawMutex> ConfigStorage<'a, M> {
    pub fn new(flash: &'a Flash<M>) -> Self {
        Self(MapStorage::new(
            flash,
            MapConfig::new(STORAGE_PARTITION_RANGE),
            NoCache::new(),
        ))
    }

    /// Сохраняет параметры инициализации механики.
    pub async fn store_mechanics(
        &mut self,
        config: StartupMechanicsConfig,
    ) -> Result<(), sequential_storage::Error<<&Flash<M> as ErrorType>::Error>> {
        let mut buf = [0u8; utils::buffer_size::<StartupMechanicsConfig>()];
        self.0
            .store_item(
                &mut buf,
                &(ConfigKey::Mechanics as u8),
                &MechanicsEntry(config),
            )
            .await
    }

    /// Загружает параметры инициализации механики. Возвращает `StorageError::NotFound`, если данных нет.
    pub async fn fetch_mechanics(
        &mut self,
    ) -> Result<StartupMechanicsConfig, StorageError<<&Flash<M> as ErrorType>::Error>> {
        let mut buf = [0u8; utils::buffer_size::<StartupMechanicsConfig>()];

        let result: Option<MechanicsEntry> = self
            .0
            .fetch_item(&mut buf, &(ConfigKey::Mechanics as u8))
            .await
            .map_err(StorageError::Flash)?;

        result.map(|w| w.0).ok_or(StorageError::NotFound)
    }

    /// Сохраняет конфигурацию Wi-Fi.
    pub async fn store_wifi(
        &mut self,
        config: WifiConfig,
    ) -> Result<(), sequential_storage::Error<<&Flash<M> as ErrorType>::Error>> {
        let mut buf = [0u8; utils::buffer_size::<WifiConfig>()];
        self.0
            .store_item(&mut buf, &(ConfigKey::Wifi as u8), &WifiEntry(config))
            .await
    }

    /// Загружает конфигурацию Wi-Fi. Возвращает `StorageError::NotFound`, если данных нет.
    pub async fn fetch_wifi(
        &mut self,
    ) -> Result<WifiConfig, StorageError<<&Flash<M> as ErrorType>::Error>> {
        let mut buf = [0u8; utils::buffer_size::<WifiConfig>()];

        let result: Option<WifiEntry> = self
            .0
            .fetch_item(&mut buf, &(ConfigKey::Wifi as u8))
            .await
            .map_err(StorageError::Flash)?;

        result.map(|w| w.0).ok_or(StorageError::NotFound)
    }
}
