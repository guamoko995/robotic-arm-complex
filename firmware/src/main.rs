#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

mod connectors;
mod core_0;
mod core_1;
mod utils;

use crate::connectors::Connectors;
use core_0::Core0;
use core_1::Core1;
use embassy_executor::Spawner;
use esp_hal::{Config, clock::CpuClock, peripherals::Peripherals};
use esp_println::println;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    let Peripherals {
        CPU_CTRL,
        LEDC,
        GPIO32,
        GPIO33,
        GPIO25,
        GPIO26,
        TIMG0,
        FLASH,
        WIFI,
        ..
    } = esp_hal::init(Config::default().with_cpu_clock(CpuClock::max()));
    let Connectors { pos, pos_ack } = Connectors::new();

    let _g = Core1::make(CPU_CTRL, LEDC, GPIO32, GPIO33, GPIO25, GPIO26)
        .run(pos.receiver(), pos_ack.sender())
        .expect("failed to start core_1");

    Core0::make(TIMG0, FLASH, WIFI)
        .run(pos.sender(), pos_ack.receiver())
        .await;
}
