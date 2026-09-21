//! 外につながる 2 つのポートのリンクを読み、変化をログに出す。

use crate::dp83867::Dp83867;
use crate::memory_map::PORT_STATS;
use crate::ports::PORT_RMII;
use crate::satcat5::port_stats::RMII_STATUS_LOCK;

pub struct Links {
    phy: Dp83867,
    dp83867_up: bool,
    /// LAN8720 の MDIO は PMOD に出していないため、リンクの代わりに REF_CLK のロックを見る。
    rmii_locked: bool,
}

impl Links {
    pub fn new(phy: Dp83867) -> Self {
        Links {
            phy,
            dp83867_up: false,
            rmii_locked: false,
        }
    }

    /// リンクは MDIO で読むため、主ループから間隔をあけて呼ぶ。
    pub fn check(&mut self) {
        let status = self.phy.status();
        if status.link != self.dp83867_up {
            self.dp83867_up = status.link;
            if status.link {
                defmt::info!(
                    "DP83867 link up, {=u32} Mbps, full duplex {=bool}",
                    status.speed_mbps,
                    status.full_duplex
                );
            } else {
                defmt::info!("DP83867 link down");
            }
        }
        let locked = PORT_STATS.link(PORT_RMII).1 & RMII_STATUS_LOCK != 0;
        if locked != self.rmii_locked {
            self.rmii_locked = locked;
            if locked {
                defmt::info!("RMII REF_CLK locked");
            } else {
                defmt::warn!("RMII REF_CLK lost");
            }
        }
    }

    pub fn dp83867_up(&self) -> bool {
        self.dp83867_up
    }
}
