//! ボードの Ethernet PHY である DP83867 を、MDIO で設定し、状態を読む。

mod registers;

use crate::satcat5::mdio::Mdio;
use registers::*;

// RGMIICTL は、RGMII を有効にし、送信と受信のクロックをデータに対してずらす設定にする。
const RGMIICTL_VALUE: u16 = 0x00D3;

// RGMIIDCTL は、送信と受信のクロックのずれを 2.00 ns にする。
// FPGA 側ではクロックをずらさないため、ずれは PHY だけで作る。
const RGMIIDCTL_VALUE: u16 = 0x0077;

// スイッチコアは 25 MHz で 200 Mbps までしか扱えないため、1000BASE-T を広告させない。
// CFG1 のビット 9 と 8 が 1000BASE-T の全二重と半二重の広告で、両方を落とす。
// 広告を変えた後は、BMCR で自動交渉をやり直させる。
const CFG1_VALUE: u16 = 0x0000;
const BMCR_VALUE: u16 = 0x1200;

const SPEEDS_MBPS: [u32; 3] = [10, 100, 1000];

pub struct Status {
    pub link: bool,
    pub speed_mbps: u32,
    pub full_duplex: bool,
}

#[derive(Clone, Copy)]
pub struct Dp83867 {
    mdio: Mdio,
    phy_addr: u32,
}

impl Dp83867 {
    pub const fn new(mdio: Mdio, phy_addr: u32) -> Self {
        Dp83867 { mdio, phy_addr }
    }

    fn write(&self, register: u32, value: u16) {
        self.mdio.write(self.phy_addr, register, value);
    }

    /// 拡張レジスタは、REGCR と ADDAR への 4 回の書き込みで、番号を指してから値を書く。
    fn write_extended(&self, register: u16, value: u16) {
        self.write(REGCR, REGCR_ADDRESS);
        self.write(ADDAR, register);
        self.write(REGCR, REGCR_DATA);
        self.write(ADDAR, value);
    }

    /// PHY のリセットの解除から、MDIO を使えるまで待った後に呼ぶ。
    pub fn init(&self) {
        self.write_extended(RGMIICTL, RGMIICTL_VALUE);
        self.write_extended(RGMIIDCTL, RGMIIDCTL_VALUE);
        self.write(CFG1, CFG1_VALUE);
        self.write(BMCR, BMCR_VALUE);
    }

    pub fn status(&self) -> Status {
        let physts = self.mdio.read(self.phy_addr, PHYSTS);
        let speed = (physts >> PHYSTS_SPEED_SHIFT) as usize;
        Status {
            link: physts & PHYSTS_LINK != 0,
            speed_mbps: SPEEDS_MBPS.get(speed).copied().unwrap_or(0),
            full_duplex: physts & PHYSTS_DUPLEX != 0,
        }
    }
}
