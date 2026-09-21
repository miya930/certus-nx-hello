//! ポートごとの統計とリンクの状態。

use super::Switch;
use embedded_hal::delay::DelayNs;
use satcat5_pac::port_stats;

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

/// リンクの状態は、取り込みを待たずに今の値を返す。
pub struct Link {
    pub speed_mbps: u32,
    pub status: u32,
}

impl Switch {
    fn stats(&self, port: usize) -> &port_stats::Port {
        unsafe { &*self.stats }.port(port)
    }

    /// 全ポートの統計を取り込む。
    pub fn refresh_stats(&self, delay: &mut impl DelayNs) {
        self.stats(0).bcast_bytes().write(|w| w.set(0));
        delay.delay_us(REFRESH_WAIT_USEC);
    }

    pub fn port_counts(&self, port: usize) -> Counts {
        let stats = self.stats(port);
        let errors = stats.errors().read();
        Counts {
            rx_broadcast_frames: stats.bcast_frames().read().bits(),
            rx_bytes: stats.rcvd_bytes().read().bits(),
            rx_frames: stats.rcvd_frames().read().bits(),
            tx_bytes: stats.sent_bytes().read().bits(),
            tx_frames: stats.sent_frames().read().bits(),
            discards: u32::from(errors.ovr_tx().bits()) + u32::from(errors.ovr_rx().bits()),
            errors: u32::from(errors.mii().bits()) + u32::from(errors.pkt().bits()),
        }
    }

    pub fn port_link(&self, port: usize) -> Link {
        let link = self.stats(port).link().read();
        Link { speed_mbps: link.speed().bits().into(), status: link.status().bits().into() }
    }
}
