//! SatCat5 の ConfigBus のデバイスを、レジスタの読み書きで操作する。
//! レジスタの番号と意味は、デバイスごとのモジュールに置く。

pub mod mailmap;
pub mod mdio;
pub mod port_stats;
pub mod switch_core;

use core::ptr::{read_volatile, write_volatile};

/// デバイスごとに 1024 個のレジスタがあり、デバイス番号を 12 ビット左に寄せた位置に並ぶ。
const DEVICE_SHIFT: usize = 12;
const REGISTER_BYTES: usize = 4;

/// ConfigBus の 1 つのデバイス。レジスタは 32 ビットで、番号の 4 倍の位置にある。
#[derive(Clone, Copy)]
pub struct Device {
    base: usize,
}

impl Device {
    pub const fn new(configbus_base: usize, number: usize) -> Self {
        Device {
            base: configbus_base + (number << DEVICE_SHIFT),
        }
    }

    pub fn read(&self, register: usize) -> u32 {
        unsafe { read_volatile((self.base + register * REGISTER_BYTES) as *const u32) }
    }

    pub fn write(&self, register: usize, value: u32) {
        unsafe { write_volatile((self.base + register * REGISTER_BYTES) as *mut u32, value) };
    }
}
