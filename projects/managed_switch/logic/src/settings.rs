//! 設定の値と、SPI Flash に保存するレコードの書式。

use crate::ports::PORT_NAMES;

#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Settings {
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub prefix: u8,
    pub gateway: Option<[u8; 4]>,
    /// 全てのフレームを出すポートの番号。
    pub mirror: Option<u8>,
}

/// MAC はローカル管理のアドレスを使い、IP はプライベートアドレスの範囲から選ぶ。
pub const DEFAULT: Settings = Settings {
    mac: [0x5A, 0x5A, 0x00, 0x00, 0x00, 0x02],
    ip: [192, 168, 1, 10],
    prefix: 24,
    gateway: None,
    mirror: None,
};

/// 書式を変えたときは末尾の番号を上げ、古い書式を読まないようにする。
const MAGIC: [u8; 4] = *b"MSW1";
/// 値がないことを表すバイト。消した Flash の値と同じにする。
const NONE: u8 = 0xFF;

// 書式: MAGIC 4、MAC 6、IP 4、プレフィックス長 1、ゲートウェイの有無 1、ゲートウェイ 4、ミラーのポート 1、CRC-32 4。
pub const RECORD_BYTES: usize = 25;
const CRC_BYTES: usize = 4;

pub type Record = [u8; RECORD_BYTES];

impl Settings {
    /// CRC-32 は Ethernet の FCS と同じ多項式を、ビットごとに計算する。
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    pub fn encode(&self) -> Record {
        let mut record = [0; RECORD_BYTES];
        record[0..4].copy_from_slice(&MAGIC);
        record[4..10].copy_from_slice(&self.mac);
        record[10..14].copy_from_slice(&self.ip);
        record[14] = self.prefix;
        record[15] = if self.gateway.is_some() { 1 } else { NONE };
        record[16..20].copy_from_slice(&self.gateway.unwrap_or([NONE; 4]));
        record[20] = self.mirror.unwrap_or(NONE);
        let crc = Self::crc32(&record[..RECORD_BYTES - CRC_BYTES]);
        record[RECORD_BYTES - CRC_BYTES..].copy_from_slice(&crc.to_le_bytes());
        record
    }

    /// 保存したことがない、または壊れているときは None を返す。
    pub fn decode(record: &Record) -> Option<Settings> {
        let (body, crc) = record.split_at(RECORD_BYTES - CRC_BYTES);
        if body[0..4] != MAGIC || Self::crc32(body).to_le_bytes() != crc {
            return None;
        }
        let mirror = match record[20] {
            NONE => None,
            port if (port as usize) < PORT_NAMES.len() => Some(port),
            _ => return None,
        };
        Some(Settings {
            mac: record[4..10].try_into().ok()?,
            ip: record[10..14].try_into().ok()?,
            prefix: record[14],
            gateway: if record[15] == NONE {
                None
            } else {
                record[16..20].try_into().ok()
            },
            mirror,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUSTOM: Settings = Settings {
        mac: [0x02, 0x00, 0x00, 0x00, 0x00, 0x01],
        ip: [10, 0, 0, 2],
        prefix: 8,
        gateway: Some([10, 0, 0, 1]),
        mirror: Some(1),
    };

    /// 本体を書き換えたあと、CRC を付け直したレコード。CRC は正しいが中身の違うレコードを作る。
    fn with_body(settings: &Settings, change: impl FnOnce(&mut Record)) -> Record {
        let mut record = settings.encode();
        change(&mut record);
        let crc = Settings::crc32(&record[..RECORD_BYTES - CRC_BYTES]);
        record[RECORD_BYTES - CRC_BYTES..].copy_from_slice(&crc.to_le_bytes());
        record
    }

    #[test]
    fn crc32_matches_the_ethernet_check_value() {
        assert_eq!(Settings::crc32(b"123456789"), 0xCBF4_3926);
    }

    /// Flash に保存済みの設定を読めなくならないよう、既定値のバイト列を固定する。
    #[test]
    fn default_record_keeps_its_layout() {
        let expected: Record = [
            b'M', b'S', b'W', b'1', // MAGIC
            0x5A, 0x5A, 0x00, 0x00, 0x00, 0x02, // MAC
            192, 168, 1, 10,   // IP
            24,   // プレフィックス長
            NONE, // ゲートウェイなし
            NONE, NONE, NONE, NONE, // ゲートウェイ
            NONE, // ミラーなし
            0xA3, 0x71, 0x3D, 0x7E, // CRC-32
        ];
        assert_eq!(DEFAULT.encode(), expected);
    }

    #[test]
    fn decodes_what_it_encodes() {
        assert_eq!(Settings::decode(&DEFAULT.encode()), Some(DEFAULT));
        assert_eq!(Settings::decode(&CUSTOM.encode()), Some(CUSTOM));
    }

    #[test]
    fn rejects_an_erased_record() {
        assert_eq!(Settings::decode(&[NONE; RECORD_BYTES]), None);
    }

    #[test]
    fn rejects_any_changed_byte() {
        let record = CUSTOM.encode();
        for index in 0..RECORD_BYTES {
            let mut broken = record;
            broken[index] ^= 0x01;
            assert_eq!(Settings::decode(&broken), None, "byte {index}");
        }
    }

    #[test]
    fn rejects_another_format_even_with_a_valid_crc() {
        let record = with_body(&DEFAULT, |record| record[3] = b'2');
        assert_eq!(Settings::decode(&record), None);
    }

    #[test]
    fn rejects_a_mirror_port_that_does_not_exist() {
        let record = with_body(&DEFAULT, |record| record[20] = PORT_NAMES.len() as u8);
        assert_eq!(Settings::decode(&record), None);
    }

    #[test]
    fn reads_every_existing_mirror_port() {
        for port in 0..PORT_NAMES.len() as u8 {
            let settings = Settings {
                mirror: Some(port),
                ..DEFAULT
            };
            assert_eq!(Settings::decode(&settings.encode()), Some(settings));
        }
    }

    #[test]
    fn ignores_the_gateway_bytes_without_the_gateway_flag() {
        let record = with_body(&DEFAULT, |record| {
            record[16..20].copy_from_slice(&[10, 0, 0, 1])
        });
        assert_eq!(
            Settings::decode(&record).map(|settings| settings.gateway),
            Some(None)
        );
    }
}
