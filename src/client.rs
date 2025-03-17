// client.rs
// Модуль клієнтської частини. Виконує читання файлу, розбиття на пакети, передачу через UDP і обробку підтверджень.

use crate::packet::Packet;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};

const PACKET_SIZE: usize = 4096; // Розмір даних у кожному пакеті (без заголовка)
const HEADER_SIZE: usize = 12;   // 3 поля по 4 байти (file_id, packet_number, total_packets)
const TIMEOUT_MS: u64 = 500;       // Тайм-аут для підтверджень
const MAX_RETRIES: u32 = 5;        // Максимальна кількість повторних передач

// Функція для читання файлу і розбиття його на пакети.
async fn read_file_and_split(file_path: &str, file_id: u32) -> io::Result<Vec<Packet>> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let file_size = buffer.len();
    let mut packets = Vec::new();
    // Обчислюємо кількість пакетів, округлюючи в більшу сторону
    let total_packets = ((file_size as f64) / (PACKET_SIZE as f64)).ceil() as u32;

    for i in 0..total_packets {
        let start = (i as usize) * PACKET_SIZE;
        let end = std::cmp::min(start + PACKET_SIZE, file_size);
        let payload = buffer[start..end].to_vec();
        let packet = Packet {
            file_id,
            packet_number: i,
            total_packets,
            payload,
        };
        packets.push(packet);
    }
    Ok(packets)
}

// Основна функція клієнтської частини.
pub async fn run_client(server_addr: SocketAddr, file_path: String) {
    println!("Запуск клієнта для передачі файлу: {}", file_path);
    // Створюємо UDP-сокет. Біндимо на довільну локальну адресу.
    let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let socket = UdpSocket::bind(local_addr).await.expect("Не вдалося створити сокет");

    // Призначаємо унікальний file_id (наприклад, випадковий або на основі часу)
    let file_id = 1001; // для прикладу

    // Розбиваємо файл на пакети
    let packets = read_file_and_split(&file_path, file_id).await.expect("Помилка читання файлу");
    println!("Файл розбитий на {} пакетів", packets.len());

    // Створюємо мапу для відстеження підтверджених пакетів
    let mut acked: HashMap<u32, bool> = HashMap::new();
    for i in 0..packets.len() {
        acked.insert(i as u32, false);
    }

    // Надсилаємо пакети та очікуємо підтвердження
    let mut retries: HashMap<u32, u32> = HashMap::new();
    for i in 0..packets.len() {
        retries.insert(i as u32, 0);
    }

    // Відправка пакетів із повторною передачею
    let mut pending_packets = packets.clone();

    while pending_packets.len() > 0 {
        // Надсилаємо всі невідправлені або не підтверджені пакети
        for packet in pending_packets.iter() {
            let data = packet.serialize().expect("Серіалізація не вдалася");
            if let Err(e) = socket.send_to(&data, server_addr).await {
                eprintln!("Помилка надсилання пакета {}: {}", packet.packet_number, e);
            } else {
                println!("Надіслано пакет {} з {} пакетів", packet.packet_number + 1, packet.total_packets);
            }
        }

        // Чекаємо на підтвердження протягом TIMEOUT_MS мілісекунд
        sleep(Duration::from_millis(TIMEOUT_MS)).await;

        // Обробка отриманих підтверджень
        let mut buf = [0u8; 8];
        // Використовуємо тайм-аут для отримання ACK
        while let Ok((size, addr)) = socket.try_recv_from(&mut buf) {
            if size >= 4 {
                // Очікуємо, що ACK містить 4 байти (номер пакета)
                let ack_packet_number = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                println!("Отримано ACK для пакета {}", ack_packet_number);
                acked.insert(ack_packet_number, true);
            }
        }

        // Фільтруємо pending_packets, залишаючи лише ті, що не підтверджені
        pending_packets = packets
            .clone()
            .into_iter()
            .filter(|p| !acked.get(&p.packet_number).unwrap_or(&false))
            .collect();

        // Збільшуємо лічильники повторних спроб
        for packet in pending_packets.iter() {
            let count = retries.entry(packet.packet_number).or_insert(0);
            *count += 1;
            if *count > MAX_RETRIES {
                eprintln!("Пакет {} не вдалося доставити після {} спроб", packet.packet_number, MAX_RETRIES);
                // Видаляємо цей пакет з pending_packets (можна обробити помилку)
            }
        }
    }

    // Після успішного надсилання всіх пакетів, надсилаємо фінальний сигнал завершення
    let finish_signal = b"FINISH";
    socket.send_to(finish_signal, server_addr).await.expect("Не вдалося надіслати сигнал завершення");
    println!("Передача файлу завершена");
}
