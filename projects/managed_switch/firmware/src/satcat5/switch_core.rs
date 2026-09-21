//! SatCat5 のスイッチコアの構成と、MAC アドレステーブル。

use super::Device;

const REG_PORT_COUNT: usize = 0;
const REG_DATA_WIDTH: usize = 1;
const REG_CORE_CLOCK: usize = 2;
const REG_TABLE_SIZE: usize = 3;
const REG_PROMISCUOUS: usize = 4;
const REG_FRAME_SIZE: usize = 7;
const REG_QUERY_MAC_LSB: usize = 11;
const REG_QUERY_MAC_MSB: usize = 12;
const REG_QUERY_CTRL: usize = 13;

// MAC アドレステーブルは、QUERY_CTRL の上位 8 ビットに操作を書いて読み書きする。
// 操作の間は QUERY_CTRL を読むと操作が返り、終わると 0 になる。
// 下位 16 ビットには、読み出しなら表の番号を書き、終わるとその項目のポートの番号が返る。
const QUERY_OP_SHIFT: u32 = 24;
const QUERY_OP_READ: u32 = 0x01;
const QUERY_OP_CLEAR: u32 = 0x03;
const QUERY_ARG_MASK: u32 = 0xFFFF;
/// 空の項目は、MAC アドレスが全て 1 で返る。
const MAC_EMPTY: [u8; 6] = [0xFF; 6];

pub struct Info {
    pub ports: u32,
    pub data_bits: u32,
    pub core_hz: u32,
    pub table_size: u32,
    pub frame_min: u32,
    pub frame_max: u32,
}

#[derive(Clone, Copy)]
pub struct SwitchCore {
    device: Device,
}

impl SwitchCore {
    pub const fn new(device: Device) -> Self {
        SwitchCore { device }
    }

    /// フレームの長さの上限は上位 16 ビットに、下限は下位 16 ビットにある。
    pub fn info(&self) -> Info {
        let frame_size = self.device.read(REG_FRAME_SIZE);
        Info {
            ports: self.device.read(REG_PORT_COUNT),
            data_bits: self.device.read(REG_DATA_WIDTH),
            core_hz: self.device.read(REG_CORE_CLOCK),
            table_size: self.device.read(REG_TABLE_SIZE),
            frame_min: frame_size & 0xFFFF,
            frame_max: frame_size >> 16,
        }
    }

    /// プロミスキャスにしたポートには、宛先によらず全てのフレームが出る。
    pub fn set_promiscuous(&self, port_mask: u32) {
        self.device.write(REG_PROMISCUOUS, port_mask);
    }

    fn query(&self, op: u32, argument: u32) -> u32 {
        while self.device.read(REG_QUERY_CTRL) >> QUERY_OP_SHIFT != 0 {}
        self.device.write(REG_QUERY_CTRL, op << QUERY_OP_SHIFT | argument);
        loop {
            let ctrl = self.device.read(REG_QUERY_CTRL);
            if ctrl >> QUERY_OP_SHIFT == 0 {
                return ctrl & QUERY_ARG_MASK;
            }
        }
    }

    /// MAC アドレスは、上位 2 バイトが QUERY_MAC_MSB の下位 16 ビットに、下位 4 バイトが QUERY_MAC_LSB に入る。
    pub fn mac_entry(&self, index: u32) -> Option<([u8; 6], usize)> {
        let port = self.query(QUERY_OP_READ, index) as usize;
        let msb = self.device.read(REG_QUERY_MAC_MSB) as u16;
        let lsb = self.device.read(REG_QUERY_MAC_LSB);
        let mut mac = [0; 6];
        mac[..2].copy_from_slice(&msb.to_be_bytes());
        mac[2..].copy_from_slice(&lsb.to_be_bytes());
        (mac != MAC_EMPTY).then_some((mac, port))
    }

    pub fn mac_clear(&self) {
        self.query(QUERY_OP_CLEAR, 0);
    }
}
