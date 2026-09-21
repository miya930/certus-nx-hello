//! SatCat5 のスイッチコアと、ポートごとの統計を ConfigBus から読み書きする。

use crate::cfgbus::{self, DEV_STATS, DEV_SWITCH};
use crate::clint;

/// ポートの番号と名前は、managed_switch.vhd の PORT_* の並びに合わせる。
pub const PORT_NAMES: [&str; 3] = ["rgmii", "rmii", "cpu"];
pub const PORT_COUNT: usize = PORT_NAMES.len();
pub const PORT_RGMII: usize = 0;
pub const PORT_RMII: usize = 1;

// スイッチコアのレジスタ。
const REG_PORT_COUNT: usize = 0;
const REG_DATA_WIDTH: usize = 1;
const REG_CORE_CLOCK: usize = 2;
const REG_TABLE_SIZE: usize = 3;
const REG_PROMISCUOUS: usize = 4;
const REG_FRAME_SIZE: usize = 7;
const REG_QUERY_MAC_LSB: usize = 11;
const REG_QUERY_MAC_MSB: usize = 12;
const REG_QUERY_CTRL: usize = 13;

pub struct Info {
    pub ports: u32,
    pub data_bits: u32,
    pub core_hz: u32,
    pub table_size: u32,
    pub frame_min: u32,
    pub frame_max: u32,
}

/// フレームの長さの上限は上位 16 ビットに、下限は下位 16 ビットにある。
pub fn info() -> Info {
    let frame_size = cfgbus::read(DEV_SWITCH, REG_FRAME_SIZE);
    Info {
        ports: cfgbus::read(DEV_SWITCH, REG_PORT_COUNT),
        data_bits: cfgbus::read(DEV_SWITCH, REG_DATA_WIDTH),
        core_hz: cfgbus::read(DEV_SWITCH, REG_CORE_CLOCK),
        table_size: cfgbus::read(DEV_SWITCH, REG_TABLE_SIZE),
        frame_min: frame_size & 0xFFFF,
        frame_max: frame_size >> 16,
    }
}

/// プロミスキャスにしたポートには、宛先によらず全てのフレームが出る。
/// ミラーリングは、指定した 1 つのポートだけをプロミスキャスにして実現する。
pub fn set_mirror(port: Option<u8>) {
    let mask = port.map_or(0, |port| 1 << port);
    cfgbus::write(DEV_SWITCH, REG_PROMISCUOUS, mask);
}

// MAC アドレステーブルは、QUERY_CTRL の上位 8 ビットに操作を書いて読み書きする。
// 操作の間は QUERY_CTRL を読むと操作が返り、終わると 0 になる。
// 下位 16 ビットには、読み出しなら表の番号を書き、終わるとその項目のポートの番号が返る。
const QUERY_OP_SHIFT: u32 = 24;
const QUERY_OP_READ: u32 = 0x01;
const QUERY_OP_CLEAR: u32 = 0x03;
const QUERY_ARG_MASK: u32 = 0xFFFF;
/// 空の項目は、MAC アドレスが全て 1 で返る。
const MAC_EMPTY: [u8; 6] = [0xFF; 6];

fn query(op: u32, argument: u32) -> u32 {
    while cfgbus::read(DEV_SWITCH, REG_QUERY_CTRL) >> QUERY_OP_SHIFT != 0 {}
    cfgbus::write(DEV_SWITCH, REG_QUERY_CTRL, op << QUERY_OP_SHIFT | argument);
    loop {
        let ctrl = cfgbus::read(DEV_SWITCH, REG_QUERY_CTRL);
        if ctrl >> QUERY_OP_SHIFT == 0 {
            return ctrl & QUERY_ARG_MASK;
        }
    }
}

/// MAC アドレスは、上位 2 バイトが QUERY_MAC_MSB の下位 16 ビットに、下位 4 バイトが QUERY_MAC_LSB に入る。
pub fn mac_entry(index: u32) -> Option<([u8; 6], usize)> {
    let port = query(QUERY_OP_READ, index) as usize;
    let msb = cfgbus::read(DEV_SWITCH, REG_QUERY_MAC_MSB) as u16;
    let lsb = cfgbus::read(DEV_SWITCH, REG_QUERY_MAC_LSB);
    let mut mac = [0; 6];
    mac[..2].copy_from_slice(&msb.to_be_bytes());
    mac[2..].copy_from_slice(&lsb.to_be_bytes());
    (mac != MAC_EMPTY).then_some((mac, port))
}

pub fn mac_clear() {
    query(QUERY_OP_CLEAR, 0);
}

// 統計はポートごとに 16 個のレジスタを持つ。
const STATS_PER_PORT: usize = 16;
pub const STAT_RX_BROADCAST_FRAMES: usize = 1;
pub const STAT_RX_BYTES: usize = 2;
pub const STAT_RX_FRAMES: usize = 3;
pub const STAT_TX_BYTES: usize = 4;
pub const STAT_TX_FRAMES: usize = 5;
pub const STAT_ERRORS: usize = 6;
pub const STAT_LINK: usize = 8;

/// どのレジスタに書いても、全ポートの数が取り込まれ、数え直しが始まる。
/// 取り込みは各ポートのクロックで行われるため、書いた後は少し待ってから読む。
const REFRESH_WAIT_USEC: u32 = 100;

pub fn refresh_stats() {
    cfgbus::write(DEV_STATS, 0, 0);
    clint::delay_us(REFRESH_WAIT_USEC);
}

pub fn stat(port: usize, register: usize) -> u32 {
    cfgbus::read(DEV_STATS, port * STATS_PER_PORT + register)
}

// リンクのレジスタは、上位 16 ビットが速度 (Mbps)、下位 8 ビットがポートの状態を示す。
// RMII のポートの状態は、ビット 1 がクロックのロック、ビット 2 が 100 Mbps であることを示す。
pub const RMII_STATUS_LOCK: u32 = 1 << 1;

pub fn link_speed_mbps(port: usize) -> u32 {
    stat(port, STAT_LINK) >> 16
}

pub fn link_status(port: usize) -> u32 {
    stat(port, STAT_LINK) & 0xFF
}
