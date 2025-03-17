// packet.rs
// Модуль для роботи з UDP-пакетами, які містять заголовок і дані файлу.

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Result};
use std::io::Read;

// Структура Packet представляє UDP-пакет із даними файлу.
#[derive(Debug, Clone)]
pub struct Packet {
    pub file_id: u32,         // Унікальний ідентифікатор файлу
    pub packet_number: u32,   // Номер поточного пакета
    pub total_packets: u32,   // Загальна кількість пакетів
    pub payload: Vec<u8>,     // Дані (фрагмент файлу)
}

impl Packet {
    // Функція для серіалізації структури Packet у вектор байтів.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        // Записуємо поля заголовка: file_id, packet_number, total_packets
        buf.write_u32::<BigEndian>(self.file_id)?;
        buf.write_u32::<BigEndian>(self.packet_number)?;
        buf.write_u32::<BigEndian>(self.total_packets)?;
        // Додаємо payload
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    // Функція для десеріалізації вектора байтів у структуру Packet.
    pub fn deserialize(data: &[u8]) -> Result<Packet> {
        let mut cursor = Cursor::new(data);
        let file_id = cursor.read_u32::<BigEndian>()?;
        let packet_number = cursor.read_u32::<BigEndian>()?;
        let total_packets = cursor.read_u32::<BigEndian>()?;
        let mut payload = Vec::new();
        cursor.read_to_end(&mut payload)?;
        Ok(Packet {
            file_id,
            packet_number,
            total_packets,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let original = Packet {
            file_id: 1234,
            packet_number: 5,
            total_packets: 100,
            payload: vec![1, 2, 3, 4, 5],
        };
        let serialized = original.serialize().unwrap();
        let deserialized = Packet::deserialize(&serialized).unwrap();
        assert_eq!(original.file_id, deserialized.file_id);
        assert_eq!(original.packet_number, deserialized.packet_number);
        assert_eq!(original.total_packets, deserialized.total_packets);
        assert_eq!(original.payload, deserialized.payload);
    }
}
