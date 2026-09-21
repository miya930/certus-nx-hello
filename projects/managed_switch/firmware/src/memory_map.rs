//! このプロジェクトで、ConfigBus のデバイスがどこにあるか。
//! アドレスは hdl/managed_switch.vhd と合わせる。NEORV32 に内蔵の周辺のアドレスは neorv32-pac にある。

use crate::drivers::switch::{Addresses, Switch};

/// ConfigBus は、CPU の外部バスのこの位置から見える。
const CONFIGBUS_BASE: usize = 0x9000_0000;
/// デバイスごとに 1024 個のレジスタがあり、デバイス番号を 12 ビット左に寄せた位置に並ぶ。
const DEVICE_SHIFT: usize = 12;

// デバイス番号は、managed_switch.vhd の DEV_* に合わせる。
const DEV_SWITCH: usize = 0;
const DEV_MAILMAP: usize = 1;
const DEV_STATS: usize = 2;
const DEV_MDIO: usize = 3;

const fn device(number: usize) -> usize {
    CONFIGBUS_BASE + (number << DEVICE_SHIFT)
}

pub const SWITCH: Switch = Switch::new(Addresses {
    core: device(DEV_SWITCH),
    mailmap: device(DEV_MAILMAP),
    stats: device(DEV_STATS),
    mdio: device(DEV_MDIO),
});
