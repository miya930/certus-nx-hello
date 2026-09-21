//! ポートの統計を一定の間隔で取り込み、累計と直近の速さを保つ。
//! 統計のブロックは取り込みのたびに数え直すため、累計はファームウェアが足し込む。

use crate::clint;
use crate::switch::{self, PORT_COUNT};

/// 取り込みの間隔。1 秒あたりの数は 32 ビットの数に収まる。
pub const SAMPLE_MSEC: u64 = 1000;

#[derive(Clone, Copy, Default)]
pub struct Totals {
    pub rx_frames: u64,
    pub rx_broadcast: u64,
    pub rx_bytes: u64,
    pub tx_frames: u64,
    pub tx_bytes: u64,
    /// 受信と送信の FIFO があふれて捨てたフレームの数。
    pub discards: u64,
    /// MAC と PHY のエラーと、FCS や長さの誤ったフレームの数。
    pub errors: u64,
}

pub struct Traffic {
    pub totals: [Totals; PORT_COUNT],
    /// 累計を数え始めた時刻。
    pub since_msec: u64,
    last_msec: u64,
    last_interval_msec: u64,
    last_rx_bytes: [u32; PORT_COUNT],
    last_tx_bytes: [u32; PORT_COUNT],
}

// エラーのレジスタは、上位から MAC と PHY、送信 FIFO のあふれ、受信 FIFO のあふれ、フレームの誤りの順に 8 ビットずつ並ぶ。
fn split_errors(word: u32) -> (u64, u64) {
    let [phy, tx_overflow, rx_overflow, frame] = word.to_be_bytes().map(u64::from);
    (tx_overflow + rx_overflow, phy + frame)
}

impl Traffic {
    pub const fn new() -> Self {
        Traffic {
            totals: [Totals {
                rx_frames: 0,
                rx_broadcast: 0,
                rx_bytes: 0,
                tx_frames: 0,
                tx_bytes: 0,
                discards: 0,
                errors: 0,
            }; PORT_COUNT],
            since_msec: 0,
            last_msec: 0,
            last_interval_msec: 0,
            last_rx_bytes: [0; PORT_COUNT],
            last_tx_bytes: [0; PORT_COUNT],
        }
    }

    /// 前回の取り込みからの数を読み、累計に足す。
    pub fn sample(&mut self) {
        let now = clint::millis();
        switch::refresh_stats();
        for (port, totals) in self.totals.iter_mut().enumerate() {
            let rx_bytes = switch::stat(port, switch::STAT_RX_BYTES);
            let tx_bytes = switch::stat(port, switch::STAT_TX_BYTES);
            let (discards, errors) = split_errors(switch::stat(port, switch::STAT_ERRORS));
            totals.rx_frames += switch::stat(port, switch::STAT_RX_FRAMES) as u64;
            totals.rx_broadcast += switch::stat(port, switch::STAT_RX_BROADCAST_FRAMES) as u64;
            totals.rx_bytes += rx_bytes as u64;
            totals.tx_frames += switch::stat(port, switch::STAT_TX_FRAMES) as u64;
            totals.tx_bytes += tx_bytes as u64;
            totals.discards += discards;
            totals.errors += errors;
            self.last_rx_bytes[port] = rx_bytes;
            self.last_tx_bytes[port] = tx_bytes;
        }
        self.last_interval_msec = now - self.last_msec;
        self.last_msec = now;
    }

    pub fn due(&self) -> bool {
        clint::millis() >= self.last_msec + SAMPLE_MSEC
    }

    pub fn clear(&mut self) {
        self.totals = [Totals::default(); PORT_COUNT];
        self.since_msec = clint::millis();
    }

    /// 直近の取り込みの間の速さを kbit/s で返す。1 ミリ秒あたりのビット数がそのまま kbit/s になる。
    pub fn rate_kbps(&self, port: usize) -> (u64, u64) {
        let interval = self.last_interval_msec.max(1);
        let rate = |bytes: u32| bytes as u64 * 8 / interval;
        (rate(self.last_rx_bytes[port]), rate(self.last_tx_bytes[port]))
    }
}
