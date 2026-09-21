//! CLINT のマシンタイマで、時刻を読み、待つ。
//! タイマは読むだけなので、作った後は複製して、時刻の要る所へ配る。

use crate::pac;
use embedded_hal::delay::DelayNs;

const NSEC_PER_SEC: u64 = 1_000_000_000;
const MSEC_PER_SEC: u64 = 1_000;

#[derive(Clone, Copy)]
pub struct Mtime {
    regs: &'static pac::clint::RegisterBlock,
    clk_hz: u32,
}

impl Mtime {
    /// タイマはコアのクロックで進むため、時間はクロック周波数から求める。
    pub fn new(_clint: pac::Clint, clk_hz: u32) -> Self {
        Mtime { regs: unsafe { &*pac::Clint::ptr() }, clk_hz }
    }

    /// タイマは 64 ビットで、下位を読む間に上位が繰り上がることがある。
    /// 上位が変わらなかったときの組み合わせだけを使う。
    pub fn ticks(&self) -> u64 {
        loop {
            let high = self.regs.mtime_hi().read().bits();
            let low = self.regs.mtime_low().read().bits();
            if high == self.regs.mtime_hi().read().bits() {
                return (high as u64) << 32 | low as u64;
            }
        }
    }

    pub fn millis(&self) -> u64 {
        self.ticks() * MSEC_PER_SEC / self.clk_hz as u64
    }
}

impl DelayNs for Mtime {
    fn delay_ns(&mut self, ns: u32) {
        let end = self.ticks() + ns as u64 * self.clk_hz as u64 / NSEC_PER_SEC;
        while self.ticks() < end {}
    }
}
