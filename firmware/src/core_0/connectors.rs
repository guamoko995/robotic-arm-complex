use common::{request::Command, wifi_config::WifiConfig};
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
    signal::Signal,
};

use crate::mk_static;

// Канал для передачи команд от сетевого API обработчику команд.
pub type CmdChan = Channel<NoopRawMutex, Command, 1>;
pub type CmdSender<'a> = Sender<'a, NoopRawMutex, Command, 1>;
pub type CmdReceiver<'a> = Receiver<'a, NoopRawMutex, Command, 1>;

// Канал для передачи подтверждений исполнения команд от обработчика команд
// к сетевому API.
pub type CmdAckChan = Channel<NoopRawMutex, (), 1>;
pub type CmdAckSender<'a> = Sender<'a, NoopRawMutex, (), 1>;
pub type CmdAckReceiver<'a> = Receiver<'a, NoopRawMutex, (), 1>;

// Сигнал обновления конфига. Для передачи обновленного конфига от обработчика
// команд к сетевому менеджеру.
pub type SignalConfigUpdated = Signal<NoopRawMutex, WifiConfig>;

pub struct Connectors {
    // Канал для передачи команд от сетевого API обработчику команд.
    pub cmd: CmdChan,

    // Канал для передачи подтверждений исполнения команд от обработчика команд
    // к сетевому API.
    pub cmd_ack: CmdAckChan,

    // Сигнал обновления конфига. Для передачи обновленного конфига от обработчика
    // команд к сетевому менеджеру.
    pub config_updated: SignalConfigUpdated,
}

impl Connectors {
    pub fn new() -> &'static Self {
        mk_static!(
            Connectors,
            Self {
                cmd: Channel::new(),
                cmd_ack: Channel::new(),
                config_updated: Signal::new(),
            }
        )
    }
}
