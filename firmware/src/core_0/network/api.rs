use crate::{
    connectors::{PosAckReceiver, PosSender},
    core_0::connectors::{CmdAckReceiver, CmdSender},
};
use common::{request::Request, response::Response};
use embassy_futures::select::{Either, select};
use embedded_io_async::{Read, ReadExactError, Write};
use esp_println::println;
use postcard::experimental::max_size::MaxSize;

/// Максимально допустимые размеры пакетов на основе схем данных.
pub const MAX_READ_PACKET_SIZE: usize =
    header_size(Request::POSTCARD_MAX_SIZE) + Request::POSTCARD_MAX_SIZE;
pub const MAX_WRITE_PACKET_SIZE: usize =
    header_size(Response::POSTCARD_MAX_SIZE) + Response::POSTCARD_MAX_SIZE;

/// Обрабатывает входящие запросы от клиента.
pub async fn send_handle<W: Write>(
    mut writer: W,
    pos_ack: PosAckReceiver,
    cmd_ack: CmdAckReceiver<'_>,
) {
    loop {
        let response = match select(cmd_ack.receive(), pos_ack.receive()).await {
            Either::First(()) => Response::CommandAck,
            Either::Second(()) => Response::PositionAck,
        };

        let response = match common::to_vec::<Response, MAX_WRITE_PACKET_SIZE>(&response) {
            Ok(data) => data,
            Err(e) => {
                println!("API OUTPUT ERROR: failed to serialize response: {:?}", e);
                break;
            }
        };

        if let Err(err) = write_packet(&mut writer, &response).await {
            println!("API OUTPUT ERROR: failed to write packet: {:?}", err);
            break;
        };

        if let Err(err) = writer.flush().await {
            println!("API OUTPUT ERROR: failed to flush: {:?}", err);
            break;
        };
    }
}

/// Обрабатывает входящие запросы от клиента.
pub async fn receive_handle<R: Read>(mut reader: R, pos: PosSender, cmd: CmdSender<'_>) {
    let mut body = [0u8; MAX_READ_PACKET_SIZE];
    loop {
        let len = match read_varint(&mut reader).await {
            Ok(l) => l,
            Err(err) => {
                println!("API INPUT ERROR: failed to read packet length: {err:?}");
                break;
            }
        };

        if len > body.len() {
            println!("API INPUT ERROR: input packet too large: {len} bytes");
            break;
        }

        if let Err(err) = reader.read_exact(&mut body[..len]).await {
            println!("API INPUT ERROR: failed to read data: {err}");
            break;
        }

        match common::from_bytes::<Request>(&body[..len]) {
            Ok(Request::Enqueue(data)) => {
                if let Err(_) = pos.try_send(data) {
                    println!("API INPUT ERROR: positioning queue is full");
                    break;
                };
            }
            Ok(Request::Immediate(data)) => {
                if let Err(_) = cmd.try_send(data) {
                    println!("API INPUT ERROR: command queue is full");
                    break;
                };
            }
            Err(err) => {
                println!("API INPUT ERROR: failed to deserialize data: {err}");
                break;
            }
        };
    }
}

/// Утилита для записи пакета с varint-префиксом длины.
async fn write_packet<W: Write>(
    writer: &mut W,
    data: &[u8],
) -> Result<(), ReadExactError<W::Error>> {
    write_varint(writer, data.len()).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

/// Записывает целое число в поток в формате LEB128.
///
/// Кодирует число максимально компактно, используя от 1 до 10 байт (для 64-бит).
pub async fn write_varint<W: Write>(
    writer: &mut W,
    mut val: usize,
) -> Result<(), ReadExactError<W::Error>> {
    while val >= 0x80 {
        // Записываем 7 бит и устанавливаем бит продолжения.
        writer.write_all(&[(val as u8 & 0x7F) | 0x80]).await?;
        val >>= 7;
    }
    // Последний байт (бит продолжения равен 0).
    writer.write_all(&[val as u8]).await?;
    Ok(())
}

// Ошибка, возвращаемая [`read_varint`]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReadVarintError<E> {
    /// Значение превышает емкость целевого типа данных.
    Overflow,
    /// Ошибка, возвращаемая внутренним ридером.
    Other(E),
}

impl<E> From<E> for ReadVarintError<E> {
    fn from(err: E) -> Self {
        Self::Other(err)
    }
}

/// Читает целое число из потока в формате LEB128.
///
/// Каждый байт хранит 7 бит данных и 1 бит-флаг продолжения (MSB).
pub async fn read_varint<R: Read>(
    reader: &mut R,
) -> Result<usize, ReadVarintError<ReadExactError<R::Error>>> {
    let mut res = 0;
    let mut shift = 0;
    let mut buf = [0u8; 1];

    loop {
        reader.read_exact(&mut buf).await?;
        let byte = buf[0];

        // Извлекаем 7 бит данных.
        res |= ((byte & 0x7F) as usize) << shift;

        // Если MSB не установлен, значит это последний байт.
        if (byte & 0x80) == 0 {
            return Ok(res);
        }

        shift += 7;

        // Проверка на переполнение разрядной сетки usize.
        if shift >= (core::mem::size_of::<usize>() * 8) {
            return Err(ReadVarintError::Overflow);
        }
    }
}

/// Рассчитывает размер заголовка LEB128 для заданного значения.
pub const fn header_size(mut val: usize) -> usize {
    let mut count = 1;
    while val >= 0x80 {
        val >>= 7;
        count += 1;
    }
    count
}
