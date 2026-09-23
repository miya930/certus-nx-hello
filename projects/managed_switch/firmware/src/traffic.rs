//! ポートの統計を取り込み、ポートごとの送受信の累計と直近の速さを保つ。
//! 統計のブロックは取り込みのたびに数え直すため、累計はファームウェアが足し込む。

use crate::drivers::clock::Clock;
use crate::memory_map::SWITCH;
use managed_switch_logic::ports::{PORT_COUNT, PORT_NAMES};

/// 取り込みの間隔。1 秒あたりの数は 32 ビットの数に収まる。
pub const SAMPLE_MSEC: u64 = 1000;

#[derive(Clone, Copy, Default)]
pub struct Totals {
    pub rx_frames: u64,
    pub rx_broadcast: u64,
    pub rx_bytes: u64,
    pub tx_frames: u64,
    pub tx_bytes: u64,
    pub rx_overflows: u64,
    pub tx_overflows: u64,
    pub frame_errors: u64,
    pub mii_errors: u64,
}

impl Totals {
    /// FIFO があふれて捨てたフレームの数。
    pub fn discards(&self) -> u64 {
        self.rx_overflows + self.tx_overflows
    }

    /// MAC と PHY のエラーと、誤ったフレームの数。
    pub fn errors(&self) -> u64 {
        self.frame_errors + self.mii_errors
    }
}

pub struct Traffic {
    clock: Clock,
    pub totals: [Totals; PORT_COUNT],
    /// 累計を数え始めた時刻。
    since_msec: u64,
    last_msec: u64,
    last_interval_msec: u64,
    last_rx_bytes: [u32; PORT_COUNT],
    last_tx_bytes: [u32; PORT_COUNT],
}

impl Traffic {
    pub fn new(clock: Clock) -> Self {
        Traffic {
            clock,
            totals: [Totals::default(); PORT_COUNT],
            since_msec: 0,
            last_msec: 0,
            last_interval_msec: 0,
            last_rx_bytes: [0; PORT_COUNT],
            last_tx_bytes: [0; PORT_COUNT],
        }
    }

    /// 前回の取り込みからの数を読み、累計に足す。メインループから SAMPLE_MSEC ごとに呼ぶ。
    pub fn sample(&mut self) {
        let now = self.clock.millis();
        SWITCH.refresh_stats(&mut self.clock);
        for (port, totals) in self.totals.iter_mut().enumerate() {
            let counts = SWITCH.port_counts(port);
            if counts.frame_errors != 0
                || counts.mii_errors != 0
                || counts.rx_overflows != 0
                || counts.tx_overflows != 0
            {
                defmt::warn!(
                    "{=str} port: {=u32} frame errors, {=u32} MAC/PHY errors, {=u32} Rx overflows, {=u32} Tx overflows",
                    PORT_NAMES[port],
                    counts.frame_errors,
                    counts.mii_errors,
                    counts.rx_overflows,
                    counts.tx_overflows
                );
            }
            totals.rx_frames += counts.rx_frames as u64;
            totals.rx_broadcast += counts.rx_broadcast_frames as u64;
            totals.rx_bytes += counts.rx_bytes as u64;
            totals.tx_frames += counts.tx_frames as u64;
            totals.tx_bytes += counts.tx_bytes as u64;
            totals.rx_overflows += counts.rx_overflows as u64;
            totals.tx_overflows += counts.tx_overflows as u64;
            totals.frame_errors += counts.frame_errors as u64;
            totals.mii_errors += counts.mii_errors as u64;
            self.last_rx_bytes[port] = counts.rx_bytes;
            self.last_tx_bytes[port] = counts.tx_bytes;
        }
        self.last_interval_msec = now - self.last_msec;
        self.last_msec = now;
    }

    /// 累計を 0 に戻す。
    /// 統計のブロックに残っている前回の取り込みからの数は、先に取り込んで捨て、クリアの後の累計に入らないようにする。
    pub fn clear(&mut self) {
        self.sample();
        self.totals = [Totals::default(); PORT_COUNT];
        self.since_msec = self.last_msec;
    }

    /// 累計を数えている時間。
    pub fn counted_msec(&self) -> u64 {
        self.clock.millis() - self.since_msec
    }

    /// 直近の取り込みの間の速さを kbit/s で返す。1 ミリ秒あたりのビット数がそのまま kbit/s になる。
    pub fn rate_kbps(&self, port: usize) -> (u64, u64) {
        let interval = self.last_interval_msec.max(1);
        let rate = |bytes: u32| bytes as u64 * 8 / interval;
        (rate(self.last_rx_bytes[port]), rate(self.last_tx_bytes[port]))
    }
}
