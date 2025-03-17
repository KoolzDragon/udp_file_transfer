// server.rs
// Модуль серверної частини. Приймає UDP-пакети, надсилає підтвердження (ACK),
// збирає отримані пакети, відновлює файл та перевіряє його цілісність.

use crate::packet::Packet;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};

const BUFFER_SIZE: usize = 8192; // Розмір буфера для прийому даних
const FINISH_SIGNAL: &[u8] = b"FINISH";

// Основна функція серверної частини.
pub async fn run_server(bind_addr: SocketAddr) {
    println!("Запуск сервера на {}", bind_addr);
    let socket = UdpSocket::bind(bind_addr).await.expect("Не вдалося прив'язати сокет");

    // Структура для зберігання отриманих пакетів: ключ – номер пакета, значення – дані
    let mut received_packets: HashMap<u32, Packet> = HashMap::new();
    let mut expected_total: Option<u32> = None;
    let mut file_id: Option<u32> = None;

    let mut buf = vec![0u8; BUFFER_SIZE];

    loop {
        // Приймаємо дані від клієнта
        let (size, src) = socket.recv_from(&mut buf).await.expect("Помилка прийому");
        if size == 0 {
            continue;
        }

        // Перевіряємо, чи отримали сигнал завершення передачі
        if size >= FINISH_SIGNAL.len() && &buf[..FINISH_SIGNAL.len()] == FINISH_SIGNAL {
            println!("Отримано сигнал завершення передачі від {}", src);
            break;
        }

        // Десеріалізуємо отримані дані в структуру Packet
        match Packet::deserialize(&buf[..size]) {
            Ok(packet) => {
                println!("Отримано пакет {} від {} (файл id: {})", packet.packet_number, src, packet.file_id);
                // Встановлюємо очікувану кількість пакетів, якщо ще не встановлено
                if expected_total.is_none() {
                    expected_total = Some(packet.total_packets);
                    file_id = Some(packet.file_id);
                }
                // Зберігаємо пакет, якщо ще не отриманий
                received_packets.entry(packet.packet_number).or_insert(packet.clone());
                // Надсилаємо ACK клієнту (відправляємо номер пакета)
                let ack = packet.packet_number.to_be_bytes();
                if let Err(e) = socket.send_to(&ack, src).await {
                    eprintln!("Помилка надсилання ACK для пакета {}: {}", packet.packet_number, e);
                }
            }
            Err(e) => {
                eprintln!("Помилка десеріалізації отриманих даних: {}", e);
            }
        }
    }

    // Перевіряємо, чи отримано всі пакети
    if let Some(total) = expected_total {
        println!("Отримано {}/{} пакетів", received_packets.len(), total);
        if received_packets.len() as u32 == total {
            // Відновлюємо файл, сортуємо пакети за порядковим номером
            let mut packets: Vec<_> = received_packets.values().cloned().collect();
            packets.sort_by_key(|p| p.packet_number);
            let mut file_data = Vec::new();
            for packet in packets {
                file_data.extend(packet.payload);
            }
            // Записуємо файл у файлову систему
            let output_file = format!("received_file_{}.dat", file_id.unwrap_or(0));
            match File::create(&output_file) {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(&file_data) {
                        eprintln!("Помилка запису файлу: {}", e);
                    } else {
                        println!("Файл успішно збережено як '{}'", output_file);
                    }
                }
                Err(e) => {
                    eprintln!("Помилка створення файлу: {}", e);
                }
            }
        } else {
            eprintln!("Не всі пакети отримано. Очікувалося {}, отримано {}.", total, received_packets.len());
        }
    } else {
        eprintln!("Не вдалося визначити загальну кількість пакетів.");
    }

    // Додаткове очікування для завершення роботи (можна адаптувати)
    sleep(Duration::from_millis(500)).await;
    println!("Сервер завершує роботу.");
}
