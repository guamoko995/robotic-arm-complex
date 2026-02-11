use common::quantities::Position;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};

use crate::mk_static;

const POS_QUEUE_LEN: usize = 16;

// Канал-очередь для передачи позиций от сетевого API к позиционеру.
pub type PosChan = Channel<CriticalSectionRawMutex, Position, POS_QUEUE_LEN>;
pub type PosSender = Sender<'static, CriticalSectionRawMutex, Position, POS_QUEUE_LEN>;
pub type PosReceiver = Receiver<'static, CriticalSectionRawMutex, Position, POS_QUEUE_LEN>;

// Канал для передачи подтверждений применения позиции от позиционера
// к сетевому API.
pub type PosAckChan = Channel<CriticalSectionRawMutex, (), 1>;
pub type PosAckSender = Sender<'static, CriticalSectionRawMutex, (), 1>;
pub type PosAckReceiver = Receiver<'static, CriticalSectionRawMutex, (), 1>;

pub struct Connectors {
    // Очередь для передачи позиций от сетевого API к позиционеру.
    pub pos: PosChan,

    // Канал для передачи подтверждений применения позиции от позиционера
    // к сетевому API.
    pub pos_ack: PosAckChan,
}

impl Connectors {
    pub fn new() -> &'static Self {
        mk_static!(
            Connectors,
            Self {
                pos: Channel::new(),
                pos_ack: Channel::new(),
            }
        )
    }
}
