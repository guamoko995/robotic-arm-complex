use crate::{quantities::Position, units::RadianPerSecond, wifi_config::WifiConfig};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, MaxSize)]
pub enum Request {
    Enqueue(Position),
    Immediate(Command),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, MaxSize)]
pub enum Command {
    SetMaxSpeed(RadianPerSecond),
    ConfigureWifi(WifiConfig),
}
