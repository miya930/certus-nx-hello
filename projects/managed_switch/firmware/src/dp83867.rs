//! ボードの Ethernet PHY である DP83867 の設定と状態。MDIO の上で読み書きする。

use crate::satcat5::mdio::Mdio;

/// ボードの DP83867 は、PHY アドレス 0 で応答する。
const PHY_ADDR: u32 = 0;

const REG_BMCR: u32 = 0x00;
const REG_CFG1: u32 = 0x09;
const REG_PHYSTS: u32 = 0x11;

// 拡張レジスタは、REGCR にアドレス指定を書き、ADDAR にレジスタ番号を書き、
// REGCR にデータ指定を書き、ADDAR に値を書く、という 4 回の書き込みで操作する。
const REG_REGCR: u32 = 0x0D;
const REG_ADDAR: u32 = 0x0E;
const REGCR_ADDRESS: u16 = 0x001F;
const REGCR_DATA: u16 = 0x401F;

// RGMIICTL は、RGMII を有効にし、送信と受信のクロックをデータに対してずらす設定にする。
const REG_RGMIICTL: u16 = 0x0032;
const RGMIICTL_VALUE: u16 = 0x00D3;

// RGMIIDCTL は、送信と受信のクロックのずれを 2.00 ns にする。
// FPGA 側ではクロックをずらさないため、ずれは PHY だけで作る。
const REG_RGMIIDCTL: u16 = 0x0086;
const RGMIIDCTL_VALUE: u16 = 0x0077;

// スイッチコアは 25 MHz で 200 Mbps までしか扱えないため、1000BASE-T を広告させない。
// CFG1 のビット 9 と 8 が 1000BASE-T の全二重と半二重の広告で、両方を落とす。
// 広告を変えた後は、BMCR で自動交渉をやり直させる。
const CFG1_VALUE: u16 = 0x0000;
const BMCR_VALUE: u16 = 0x1200;

// PHYSTS のビット 15 と 14 が速度、13 が全二重、10 がリンクを示す。
const PHYSTS_SPEED_SHIFT: u16 = 14;
const PHYSTS_DUPLEX: u16 = 1 << 13;
const PHYSTS_LINK: u16 = 1 << 10;
const SPEEDS_MBPS: [u32; 3] = [10, 100, 1000];

pub struct Status {
    pub link: bool,
    pub speed_mbps: u32,
    pub full_duplex: bool,
}

fn write_extended(mdio: &Mdio, register: u16, value: u16) {
    mdio.write(PHY_ADDR, REG_REGCR, REGCR_ADDRESS);
    mdio.write(PHY_ADDR, REG_ADDAR, register);
    mdio.write(PHY_ADDR, REG_REGCR, REGCR_DATA);
    mdio.write(PHY_ADDR, REG_ADDAR, value);
}

/// PHY のリセットの解除から、MDIO を使えるまで待った後に呼ぶ。
pub fn init(mdio: &Mdio) {
    write_extended(mdio, REG_RGMIICTL, RGMIICTL_VALUE);
    write_extended(mdio, REG_RGMIIDCTL, RGMIIDCTL_VALUE);
    mdio.write(PHY_ADDR, REG_CFG1, CFG1_VALUE);
    mdio.write(PHY_ADDR, REG_BMCR, BMCR_VALUE);
}

pub fn status(mdio: &Mdio) -> Status {
    let physts = mdio.read(PHY_ADDR, REG_PHYSTS);
    let speed = (physts >> PHYSTS_SPEED_SHIFT) as usize;
    Status {
        link: physts & PHYSTS_LINK != 0,
        speed_mbps: SPEEDS_MBPS.get(speed).copied().unwrap_or(0),
        full_duplex: physts & PHYSTS_DUPLEX != 0,
    }
}
