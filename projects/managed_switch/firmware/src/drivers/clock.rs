//! CLINT のマシンタイマで時間を測る時計と、一定の周期で処理を行うための Ticker。

use embedded_hal::delay::DelayNs;
use neorv32_hal::mtime::Mtime;

/// マシンタイマは読むだけなので、この型は写して使える。
#[derive(Clone, Copy)]
pub struct Clock {
    mtime: Mtime,
}

impl Clock {
    pub fn new(mtime: Mtime) -> Self {
        Clock { mtime }
    }

    /// 起動からの時間。
    pub fn millis(&self) -> u64 {
        self.mtime.millis()
    }

    pub fn ticker(&self, period_msec: u64) -> Ticker {
        Ticker { clock: *self, period_msec, next_msec: 0 }
    }
}

impl DelayNs for Clock {
    fn delay_ns(&mut self, ns: u32) {
        self.mtime.delay_ns(ns);
    }
}

/// 主ループから毎回 due を問い合わせ、周期が来たときだけ処理を行う。
pub struct Ticker {
    clock: Clock,
    period_msec: u64,
    next_msec: u64,
}

impl Ticker {
    /// 周期が来ていれば真を返し、次の時刻を今から 1 周期後にする。
    /// 作った直後の due は真になり、最初の処理をすぐに行う。
    /// 主ループが遅れたときは、遅れた分を取り戻さずに次の周期を数え始める。
    pub fn due(&mut self) -> bool {
        let now = self.clock.millis();
        if now < self.next_msec {
            return false;
        }
        self.next_msec = now + self.period_msec;
        true
    }
}
