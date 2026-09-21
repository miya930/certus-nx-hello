//! FPGA の中の SatCat5 のスイッチングハブ。
//! スイッチコア、CPU のポート、ポートごとの統計、PHY への MDIO は、それぞれ ConfigBus の別のデバイスだが、
//! 1 つのスイッチングハブとしてまとめて扱う。

mod cpu_port;
mod mac_table;
mod mdio;
mod stats;

pub use cpu_port::CpuPort;
pub use mdio::Mdio;
pub use stats::RMII_STATUS_LOCK;

use satcat5_pac::{mdio as mdio_regs, port_mailmap, port_stats, switch_core};

/// スイッチングハブを構成する ConfigBus のデバイスの番地。
pub struct Addresses {
    pub core: usize,
    pub mailmap: usize,
    pub stats: usize,
    pub mdio: usize,
}

pub struct Info {
    pub ports: u32,
    pub data_bits: u32,
    pub core_hz: u32,
    pub table_size: u32,
    pub frame_min: u32,
    pub frame_max: u32,
}

/// レジスタは ConfigBus の番地にあるため、この型は番地だけを持ち、どこからでも写して使える。
#[derive(Clone, Copy)]
pub struct Switch {
    core: *const switch_core::RegisterBlock,
    mailmap: *const port_mailmap::RegisterBlock,
    stats: *const port_stats::RegisterBlock,
    mdio: *const mdio_regs::RegisterBlock,
}

impl Switch {
    pub const fn new(addresses: Addresses) -> Self {
        Switch {
            core: addresses.core as *const _,
            mailmap: addresses.mailmap as *const _,
            stats: addresses.stats as *const _,
            mdio: addresses.mdio as *const _,
        }
    }

    fn core(&self) -> &switch_core::RegisterBlock {
        unsafe { &*self.core }
    }

    pub fn info(&self) -> Info {
        let core = self.core();
        let frame_size = core.frame_size().read();
        Info {
            ports: core.port_count().read().bits(),
            data_bits: core.data_width().read().bits(),
            core_hz: core.core_clock().read().bits(),
            table_size: core.table_size().read().bits(),
            frame_min: frame_size.min().bits().into(),
            frame_max: frame_size.max().bits().into(),
        }
    }

    /// プロミスキャスにしたポートには、宛先によらず全てのフレームが出る。
    pub fn set_promiscuous(&self, port_mask: u32) {
        self.core().promiscuous().write(|w| w.set(port_mask));
    }

    /// CPU のポートを、smoltcp の Device として取り出す。受け取ったフレームを置く場所を持つため、1 つだけ作る。
    pub fn cpu_port(&self) -> CpuPort {
        CpuPort::new(self.mailmap)
    }

    /// PHY の設定と状態を読むための MDIO。
    pub fn mdio(&self) -> Mdio {
        Mdio::new(self.mdio)
    }
}
