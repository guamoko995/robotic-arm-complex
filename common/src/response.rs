use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, MaxSize)]
pub enum Response {
    PositionAck,
    CommandAck,
}
