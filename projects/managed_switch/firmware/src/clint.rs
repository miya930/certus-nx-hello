//! NEORV32 の CLINT にあるマシンタイマで、時間を測る。

use core::ptr::read_volatile;

const MTIME_LO: *const u32 = 0xFFF4_BFF8 as *const u32;
const MTIME_HI: *const u32 = 0xFFF4_BFFC as *const u32;

/// タイマはシステムクロックで進むため、経過時間をクロック周波数から求める。
pub const CLK_HZ: u32 = 25_000_000;
const TICKS_PER_USEC: u64 = CLK_HZ as u64 / 1_000_000;

/// マシンタイマは 64 ビットで、下位を読む間に上位が繰り上がることがある。
/// 上位が変わらなかったときの組み合わせだけを使う。
pub fn mtime() -> u64 {
    loop {
        let high = unsafe { read_volatile(MTIME_HI) };
        let low = unsafe { read_volatile(MTIME_LO) };
        if high == unsafe { read_volatile(MTIME_HI) } {
            return ((high as u64) << 32) | low as u64;
        }
    }
}

pub fn millis() -> u64 {
    mtime() / (TICKS_PER_USEC * 1000)
}

pub fn delay_us(usec: u32) {
    let end = mtime() + usec as u64 * TICKS_PER_USEC;
    while mtime() < end {}
}
