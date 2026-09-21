//! SatCat5 のポートごとの統計とリンクの状態。

use super::Device;
use embedded_hal::delay::DelayNs;

/// ポートごとに 16 個のレジスタを持つ。
const REGS_PER_PORT: usize = 16;
const STAT_RX_BROADCAST_FRAMES: usize = 1;
const STAT_RX_BYTES: usize = 2;
const STAT_RX_FRAMES: usize = 3;
const STAT_TX_BYTES: usize = 4;
const STAT_TX_FRAMES: usize = 5;
const STAT_ERRORS: usize = 6;
const STAT_LINK: usize = 8;

/// どのレジスタに書いても、全ポートの数が取り込まれ、数え直しが始まる。
/// 取り込みは各ポートのクロックで行われるため、書いた後は少し待ってから読む。
const REFRESH_WAIT_USEC: u32 = 100;

/// RMII のポートの状態は、ビット 1 がクロックのロック、ビット 2 が 100 Mbps であることを示す。
pub const RMII_STATUS_LOCK: u32 = 1 << 1;

/// 前回の取り込みから今回の取り込みまでの数。
pub struct Counts {
    pub rx_broadcast_frames: u32,
    pub rx_bytes: u32,
    pub rx_frames: u32,
    pub tx_bytes: u32,
    pub tx_frames: u32,
    /// 受信と送信の FIFO があふれて捨てたフレームの数。
    pub discards: u32,
    /// MAC と PHY のエラーと、FCS や長さの誤ったフレームの数。
    pub errors: u32,
}

#[derive(Clone, Copy)]
pub struct PortStats {
    device: Device,
}

impl PortStats {
    pub const fn new(device: Device) -> Self {
        PortStats { device }
    }

    fn read(&self, port: usize, register: usize) -> u32 {
        self.device.read(port * REGS_PER_PORT + register)
    }

    pub fn refresh(&self, delay: &mut impl DelayNs) {
        self.device.write(0, 0);
        delay.delay_us(REFRESH_WAIT_USEC);
    }

    // エラーのレジスタは、上位から MAC と PHY、送信 FIFO のあふれ、受信 FIFO のあふれ、フレームの誤りの順に 8 ビットずつ並ぶ。
    pub fn counts(&self, port: usize) -> Counts {
        let [phy, tx_overflow, rx_overflow, frame] = self.read(port, STAT_ERRORS).to_be_bytes().map(u32::from);
        Counts {
            rx_broadcast_frames: self.read(port, STAT_RX_BROADCAST_FRAMES),
            rx_bytes: self.read(port, STAT_RX_BYTES),
            rx_frames: self.read(port, STAT_RX_FRAMES),
            tx_bytes: self.read(port, STAT_TX_BYTES),
            tx_frames: self.read(port, STAT_TX_FRAMES),
            discards: tx_overflow + rx_overflow,
            errors: phy + frame,
        }
    }

    /// リンクのレジスタは取り込みを待たずに今の値を返し、上位 16 ビットが速度 (Mbps)、下位 8 ビットがポートの状態を示す。
    pub fn link(&self, port: usize) -> (u32, u32) {
        let link = self.read(port, STAT_LINK);
        (link >> 16, link & 0xFF)
    }
}
