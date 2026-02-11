//! # Async Flash Wrapper
//!
//! Обертка над блокирующим `esp-storage` для работы в асинхронной среде.

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::Mutex;
use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};
use embedded_storage_async::nor_flash::{
    ErrorType as AsyncErrorType, NorFlash as AsyncNorFlash, ReadNorFlash as AsyncReadNorFlash,
};
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;

/// Асинхронный интерфейс к Flash-памяти с настраиваемой стратегией блокировки.
///
/// `M` — реализация `RawMutex` (напр. `NoopRawMutex` или `CriticalSectionRawMutex`).
pub struct Flash<M: RawMutex> {
    storage: Mutex<M, FlashStorage>,
    capacity: usize,
}

impl<M: RawMutex> Flash<M> {
    pub fn new(flash: FLASH) -> Self {
        // Используемая версия esp_storage не использует переферию.
        // Искусственно поглощаем переферийное устройство фдэшпамяти для
        // гарантий отсутствия гонок и наглядности в вызывающем коде.
        drop(flash);
        let st = FlashStorage::new();
        let capacity = <FlashStorage as ReadNorFlash>::capacity(&st);
        Self {
            storage: Mutex::new(st),
            capacity,
        }
    }
}

impl<M: RawMutex> AsyncErrorType for &Flash<M> {
    type Error = <FlashStorage as ErrorType>::Error;
}

impl<M: RawMutex> AsyncReadNorFlash for &Flash<M> {
    const READ_SIZE: usize = <FlashStorage as ReadNorFlash>::READ_SIZE;

    fn capacity(&self) -> usize {
        self.capacity
    }

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let mut stor = self.storage.lock().await;
        <FlashStorage as ReadNorFlash>::read(&mut *stor, offset, bytes)
    }
}

impl<M: RawMutex> AsyncNorFlash for &Flash<M> {
    const WRITE_SIZE: usize = <FlashStorage as NorFlash>::WRITE_SIZE;
    const ERASE_SIZE: usize = <FlashStorage as NorFlash>::ERASE_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let mut stor = self.storage.lock().await;
        <FlashStorage as NorFlash>::erase(&mut *stor, from, to)
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let mut stor = self.storage.lock().await;
        <FlashStorage as NorFlash>::write(&mut *stor, offset, bytes)
    }
}
