//! SatCat5 の ConfigBus を、CPU の外部バスから読み書きする。

use core::ptr::{read_volatile, write_volatile};

/// ConfigBus は、CPU の外部バスのこの位置から見える。
const BASE: usize = 0x9000_0000;
/// デバイスごとに 1024 個のレジスタがあり、デバイス番号を 12 ビット左に寄せた位置に並ぶ。
const DEVICE_SHIFT: usize = 12;

// デバイス番号は、managed_switch.vhd の DEV_* に合わせる。
pub const DEV_SWITCH: usize = 0;
pub const DEV_MAILMAP: usize = 1;
pub const DEV_STATS: usize = 2;
pub const DEV_MDIO: usize = 3;

fn address(device: usize, register: usize) -> usize {
    BASE + (device << DEVICE_SHIFT) + register * 4
}

pub fn read(device: usize, register: usize) -> u32 {
    unsafe { read_volatile(address(device, register) as *const u32) }
}

pub fn write(device: usize, register: usize, value: u32) {
    unsafe { write_volatile(address(device, register) as *mut u32, value) };
}
