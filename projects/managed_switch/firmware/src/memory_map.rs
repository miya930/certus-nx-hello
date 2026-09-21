//! このプロジェクトで、ConfigBus のデバイスがどこにあるか。
//! 番地は hdl/managed_switch.vhd と合わせる。NEORV32 に内蔵の周辺の番地は neorv32-pac にある。

use crate::satcat5::{mailmap::MailMapPort, mdio::Mdio, port_stats::PortStats, switch_core::SwitchCore, Device};

/// ConfigBus は、CPU の外部バスのこの位置から見える。
const CONFIGBUS_BASE: usize = 0x9000_0000;

// デバイス番号は、managed_switch.vhd の DEV_* に合わせる。
const DEV_SWITCH: usize = 0;
const DEV_MAILMAP: usize = 1;
const DEV_STATS: usize = 2;
const DEV_MDIO: usize = 3;

pub const SWITCH_CORE: SwitchCore = SwitchCore::new(Device::new(CONFIGBUS_BASE, DEV_SWITCH));
pub const MAILMAP: MailMapPort = MailMapPort::new(Device::new(CONFIGBUS_BASE, DEV_MAILMAP));
pub const PORT_STATS: PortStats = PortStats::new(Device::new(CONFIGBUS_BASE, DEV_STATS));
pub const MDIO: Mdio = Mdio::new(Device::new(CONFIGBUS_BASE, DEV_MDIO));
