//! コンソールから変える設定と、SPI Flash への保存。

use crate::mt25q::{self, Mt25q};
use crate::ports::PORT_NAMES;
use embedded_hal::spi::SpiDevice;

#[derive(Clone, Copy, PartialEq)]
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

/// FPGA のビットストリームは Flash の先頭から置かれる。
/// 設定は、ビットストリームと重ならない最後の区画に置く。
const OFFSET: u32 = mt25q::CAPACITY_BYTES - mt25q::SUBSECTOR_BYTES;

/// 書式を変えたときは末尾の番号を上げ、古い書式を読まないようにする。
const MAGIC: [u8; 4] = *b"MSW1";
/// 値がないことを表すバイト。消した Flash の値と同じにする。
const NONE: u8 = 0xFF;

// 書式: MAGIC 4、MAC 6、IP 4、プレフィックス長 1、ゲートウェイの有無 1、ゲートウェイ 4、ミラーのポート 1、CRC-32 4。
const RECORD_BYTES: usize = 25;
const CRC_BYTES: usize = 4;
const _: () = assert!(RECORD_BYTES <= mt25q::PAGE_BYTES);

/// CRC-32 は Ethernet の FCS と同じ多項式を、ビットごとに計算する。
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

impl Settings {
    fn encode(&self) -> [u8; RECORD_BYTES] {
        let mut record = [0; RECORD_BYTES];
        record[0..4].copy_from_slice(&MAGIC);
        record[4..10].copy_from_slice(&self.mac);
        record[10..14].copy_from_slice(&self.ip);
        record[14] = self.prefix;
        record[15] = if self.gateway.is_some() { 1 } else { NONE };
        record[16..20].copy_from_slice(&self.gateway.unwrap_or([NONE; 4]));
        record[20] = self.mirror.unwrap_or(NONE);
        let crc = crc32(&record[..RECORD_BYTES - CRC_BYTES]);
        record[RECORD_BYTES - CRC_BYTES..].copy_from_slice(&crc.to_le_bytes());
        record
    }

    fn decode(record: &[u8; RECORD_BYTES]) -> Option<Settings> {
        let (body, crc) = record.split_at(RECORD_BYTES - CRC_BYTES);
        if body[0..4] != MAGIC || crc32(body).to_le_bytes() != crc {
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
            gateway: if record[15] == NONE { None } else { record[16..20].try_into().ok() },
            mirror,
        })
    }

    /// 保存したことがない、または壊れているときは None を返す。
    pub fn load<S: SpiDevice>(flash: &mut Mt25q<S>) -> Option<Settings> {
        let mut record = [0; RECORD_BYTES];
        flash.read(OFFSET, &mut record).ok()?;
        Settings::decode(&record)
    }

    pub fn save<S: SpiDevice>(&self, flash: &mut Mt25q<S>) -> Result<(), S::Error> {
        flash.erase_subsector(OFFSET)?;
        flash.program(OFFSET, &self.encode())
    }
}
