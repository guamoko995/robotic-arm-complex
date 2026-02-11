use common::{self, quantities::Position, request::Request, response::Response}; // Добавили Response
use std::env;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader}; // Убрали AsyncBufReadExt, так как читаем не строки
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;

/// Читает длину в формате LEB128 (Varint) из асинхронного потока.
async fn read_varint<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) -> io::Result<usize> {
    let mut res = 0;
    let mut shift = 0;
    loop {
        let byte = reader.read_u8().await?;
        res |= ((byte & 0x7F) as usize) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    Ok(res)
}

/// Записывает длину в формате LEB128 (Varint)
async fn write_varint<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    mut val: usize,
) -> io::Result<()> {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        writer.write_all(&[byte]).await?;
        if val == 0 {
            break;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Использование: {} <IP_ADDRESS:PORT>", args[0]);
        std::process::exit(1);
    }
    let addr = &args[1];

    let stream = TcpStream::connect(addr).await?;
    println!("Успешно подключено к {}", addr);
    println!("Команды: go <rot> <sho> <for> <cla> | exit");

    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Поток чтения stdin
    tokio::spawn(async move {
        let mut stdin_reader = BufReader::new(tokio::io::stdin());
        let mut line = String::new();
        while let Ok(n) = stdin_reader.read_line(&mut line).await {
            if n == 0 {
                break;
            }
            if tx.send(line.trim().to_string()).await.is_err() {
                break;
            }
            line.clear();
        }
    });

    // Буфер для десериализации (размер берем из MaxSize если доступно, либо с запасом)
    let mut response_buf = [0u8; 512];

    loop {
        select! {
            // Чтение ОТВЕТОВ от сервера (Response)
            len_res = read_varint(&mut reader) => {
                match len_res {
                    Ok(len) => {
                        if len > response_buf.len() {
                            eprintln!("Ошибка: слишком большой пакет от сервера ({} байт)", len);
                            break;
                        }

                        // Читаем ровно len байт
                        if let Err(e) = reader.read_exact(&mut response_buf[..len]).await {
                            eprintln!("Ошибка при чтении тела ответа: {}", e);
                            break;
                        }

                        // Десериализация postcard
                        match common::from_bytes::<Response>(&response_buf[..len]) {
                            Ok(resp) => {
                                println!("\n[СЕРВЕР] {:?}", resp);
                            }
                            Err(e) => eprintln!("Ошибка десериализации ответа: {:?}", e),
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                        println!("Соединение закрыто сервером.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Ошибка чтения длины: {}", e);
                        break;
                    }
                }
            }

            // Отправка КОМАНД на сервер
            Some(input) = rx.recv() => {
                if input.eq_ignore_ascii_case("exit") {
                    let _ = write_half.shutdown().await;
                    break;
                }

                // --- Команда управления Wi-Fi ---
                if input.starts_with("wifi ") {
                    let v: Vec<&str> = input.split_whitespace().collect();
                    if v.len() < 3 {
                        println!("Использование: wifi <SSID> <PASSWORD>");
                        continue;
                    }

                    // 1. Подготовка строк фиксированной длины
                let ssid_raw =  common::String::<{common::wifi_config::MAX_SSID_LEN}>::try_from(v[1])
                .map_err(|_| "SSID слишком длинный")?;

                let pass_raw = common::String::<{common::wifi_config::MAX_PASS_LEN}>::try_from(v[2])
                .map_err(|_| "Пароль слишком длинный")?;
                    // 2. Сборка полного конфига клиента
                    let client_cfg = common::wifi_config::ClientConfig {
                        ssid: ssid_raw.into(),
                        password: pass_raw.into(),
                        bssid: None,
                        auth_method: common::wifi_config::AuthMethod::Wpa2Personal, // По умолчанию
                        channel: None,
                        protocols: common::wifi_config::ProtocolsSet::default(), // BGN по умолчанию
                    };

                    // 3. Упаковка в WifiConfig (AP оставляем дефолтной или None)
                    let mut wifi_config = common::wifi_config::WifiConfig::default();
                    wifi_config.client = Some(client_cfg);

                    let req = Request::Immediate(common::request::Command::ConfigureWifi(wifi_config));

                    // 4. Сериализация (Postcard)
                    // Используем буфер 512, так как SSID(32) + PASS(64) + метаданные точно влезут
                    let bytes = match common::to_vec::<Request, 512>(&req) {
                        Ok(b) => b,
                        Err(e) => { println!("Ошибка сериализации: {:?}", e); continue; }
                    };

                    // 5. Отправка пакета: [Length Varint][Body Bytes]
                    if write_varint(&mut write_half, bytes.len()).await.is_ok() {
                        let _ = write_half.write_all(&bytes).await;
                        let _ = write_half.flush().await;
                        println!(">>> Команда UpdateWifi отправлена на роборуку");
                    }
                }

                if input.starts_with("go ") {
                    let v: Vec<&str> = input.split_whitespace().collect();
                    if v.len() != 5 {
                        println!("Нужно 4 координаты. Пример: go 1.5 1.0 0.5 0.0");
                        continue;
                }

                    let coords: Vec<f32> = v[1..].iter().filter_map(|s| s.parse().ok()).collect();
                if coords.len() != 4 { continue; }

                    let req = Request::Enqueue(Position {
                        rotation: coords[0].into(),
                        shoulder: coords[1].into(),
                        forearm: coords[2].into(),
                        claw: coords[3].into(),
                    });

                    let bytes = match common::to_vec::<Request, 100>(&req) {
                        Ok(b) => b,
                        Err(e) => { println!("Ошибка сериализации: {:?}", e); continue; }
                    };

                    // Отправка длины + тела
                    if write_varint(&mut write_half, bytes.len()).await.is_ok() {
                        let _ = write_half.write_all(&bytes).await;
                        let _ = write_half.flush().await;
                    }
                }
            }
        }
    }
    Ok(())
}
