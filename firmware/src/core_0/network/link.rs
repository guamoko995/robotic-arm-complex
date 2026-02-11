use crate::{core_0::network::connectors::TargetConfig, mk_static};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use esp_println::println;
use esp_radio::wifi::{self, ModeConfig, WifiController, WifiEvent, WifiStaState};

const CONNECT_RETRY_TIMEOUT: Duration = Duration::from_secs(10);
const START_RETRY_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_RETRY_TIMEOUT: Duration = Duration::from_secs(10);
const WIFI_STABILIZATION_DELAY: Duration = Duration::from_millis(1000);

pub struct WifiProvider<'a> {
    controller: WifiController<'a>,
    current_config: Option<ModeConfig>,
}

impl<'a> WifiProvider<'a> {
    pub fn new(controller: WifiController<'a>) -> Self {
        Self {
            controller,
            current_config: None,
        }
    }

    pub async fn run(&mut self, target_config_sig: &TargetConfig) -> ! {
        loop {
            // Если конфигурации еще нет, ждем её
            let new_config = if self.current_config.is_none() {
                target_config_sig.wait().await
            } else {
                // Если есть текущая — работаем и следим за обновлениями или разрывом связи
                match select(target_config_sig.wait(), self.watch_connection()).await {
                    Either::First(cfg) => cfg,
                    Either::Second(_) => {
                        // watch_connection завершился (например, потеря связи в STA)
                        // Просто перезаходим в цикл, чтобы maintain_sta_connection сработал
                        self.current_config.clone().unwrap()
                    }
                }
            };

            self.update_config(new_config).await;

            // Если в конфиге есть режим STA, обеспечиваем подключение
            if self.is_sta_active() {
                self.maintain_sta_connection().await;
            }

            Timer::after(WIFI_STABILIZATION_DELAY).await;
        }
    }

    async fn watch_connection(&mut self) {
        if self.is_sta_active() {
            if wifi::sta_state() == WifiStaState::Connected {
                // Ждем именно события разрыва
                self.controller
                    .wait_for_event(WifiEvent::StaDisconnected)
                    .await;
                println!("WIFI: Connection lost");
            }
        } else {
            // Если мы в чистом AP, просто "висим" и ждем сигнала нового конфига (select сверху это сделает)
            core::future::pending::<()>().await;
        }
    }

    async fn update_config(&mut self, new_config: ModeConfig) {
        if self.current_config.as_ref() != Some(&new_config) {
            println!("WIFI: Configuration change detected. Resetting stack...");

            // 1. Останавливаем текущую работу, если она была
            if self.current_config.is_some() {
                while let Err(e) = self.controller.stop_async().await {
                    println!("WIFI ERROR: stop failed: {:?}. Retrying...", e);
                    Timer::after(STOP_RETRY_TIMEOUT).await;
                }
            }

            // 2. Применяем новый блоб настроек
            println!("WIFI: Applying new hardware configuration...");
            if let Err(e) = self.controller.set_config(&new_config) {
                println!("WIFI ERROR: set_config failed: {:?}", e);
                // Тут можно либо паниковать, либо ждать и пробовать снова
            }

            self.current_config = Some(new_config);

            // 3. Если новый режим предполагает работу (AP или STA), стартуем
            // Запуск произойдет в maintain_sta_connection или здесь для AP
            if !matches!(self.current_config, Some(ModeConfig::None)) {
                if let Err(e) = self.controller.start_async().await {
                    println!("WIFI ERROR: start failed after config update: {:?}", e);
                }
            }

            Timer::after(WIFI_STABILIZATION_DELAY).await;
        }
    }

    fn is_sta_active(&self) -> bool {
        matches!(
            self.current_config,
            Some(ModeConfig::Client(_)) | Some(ModeConfig::ApSta(_, _))
        )
    }

    async fn maintain_sta_connection(&mut self) {
        let state = wifi::sta_state();
        match state {
            WifiStaState::Stopped => {
                if let Err(e) = self.controller.start_async().await {
                    println!("WIFI start error: {e:?}");
                    Timer::after(START_RETRY_TIMEOUT).await;
                }
            }
            WifiStaState::Started | WifiStaState::Disconnected => {
                if let Err(e) = self.controller.connect_async().await {
                    println!("WIFI connect error: {e:?}");
                    Timer::after(CONNECT_RETRY_TIMEOUT).await;
                }
            }
            _ => {} // Подключен или в процессе
        }
    }
}

/// Инициализирует Wi‑Fi/BLE контроллер и возвращает статическую ссылку на контроллер.
pub fn init_radio() -> &'static esp_radio::Controller<'static> {
    mk_static!(
        esp_radio::Controller<'static>,
        esp_radio::init().expect("WIFI: failed to init Wi-Fi/BLE controller")
    )
}
