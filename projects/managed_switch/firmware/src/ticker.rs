//! 一定の周期で処理を行うための時計。主ループから毎回 due を問い合わせる。

use neorv32_hal::mtime::Mtime;

pub struct Ticker {
    mtime: Mtime,
    period_msec: u64,
    next_msec: u64,
}

impl Ticker {
    /// 作った直後の due は真になり、最初の処理をすぐに行う。
    pub fn new(mtime: Mtime, period_msec: u64) -> Self {
        Ticker { mtime, period_msec, next_msec: 0 }
    }

    /// 周期が来ていれば真を返し、次の時刻を今から 1 周期後にする。
    /// 主ループが遅れたときは、遅れた分を取り戻さずに次の周期を数え始める。
    pub fn due(&mut self) -> bool {
        let now = self.mtime.millis();
        if now < self.next_msec {
            return false;
        }
        self.next_msec = now + self.period_msec;
        true
    }
}
