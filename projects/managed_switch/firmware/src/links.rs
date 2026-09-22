//! 外につながる 2 つのポートのリンクを読み、変化をログに出す。
//! DP83867 のリンクが変わったときは、MAC アドレステーブルを消す。

use crate::drivers::dp83867::Dp83867;
use crate::drivers::switch::RMII_STATUS_LOCK;
use crate::memory_map::SWITCH;
use managed_switch_logic::ports::PORT_RMII;

pub struct Links {
    phy: Dp83867,
    dp83867_up: bool,
    /// LAN8720 の MDIO は PMOD に出していないため、リンクの代わりに REF_CLK のロックを見る。
    rmii_locked: bool,
}

impl Links {
    pub fn new(phy: Dp83867) -> Self {
        Links { phy, dp83867_up: false, rmii_locked: false }
    }

    /// リンクは MDIO で読むため、メインループから間隔をあけて呼ぶ。
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
            // スイッチコアは、学習した MAC アドレスが別のポートから届いても、表の項目を書き換えない。
            // 機器を差し替えたときに元のポートへ送り続けないよう、表を消して再学習させる。
            // ポートは 2 つなので、どちらの向きに差し替えても DP83867 のリンクが変わる。
            SWITCH.mac_clear();
            defmt::info!("Cleared the MAC address table");
        }
        let locked = SWITCH.port_link(PORT_RMII).status & RMII_STATUS_LOCK != 0;
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
