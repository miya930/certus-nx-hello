//! ConfigBus の cfgbus_mdio で、DP83867 のレジスタを読み書きする。

use crate::cfgbus::{self, DEV_MDIO};

/// cfgbus_mdio はレジスタを 1 つだけ持つ。
const REG_MDIO: usize = 0;

// 書くと MDIO の操作を FIFO に積む。
// ビット 27 と 26 が操作、25 から 21 が PHY のアドレス、20 から 16 がレジスタ番号、15 から 0 が書く値になる。
const OP_WRITE: u32 = 0b01 << 26;
const OP_READ: u32 = 0b10 << 26;
const PHY_SHIFT: u32 = 21;
const REG_SHIFT: u32 = 16;

// 読むと、ビット 31 が操作の FIFO が満杯なこと、ビット 30 が読み出した値があることを示す。
// 読み出した値も FIFO に積まれ、このレジスタを読むたびに 1 つずつ取り出される。
const STATUS_FULL: u32 = 1 << 31;
const STATUS_VALID: u32 = 1 << 30;

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

fn command(op: u32, register: u32, value: u16) {
    while cfgbus::read(DEV_MDIO, REG_MDIO) & STATUS_FULL != 0 {}
    let word = op | PHY_ADDR << PHY_SHIFT | register << REG_SHIFT | value as u32;
    cfgbus::write(DEV_MDIO, REG_MDIO, word);
}

pub fn write(register: u32, value: u16) {
    command(OP_WRITE, register, value);
}

pub fn read(register: u32) -> u16 {
    command(OP_READ, register, 0);
    loop {
        let status = cfgbus::read(DEV_MDIO, REG_MDIO);
        if status & STATUS_VALID != 0 {
            return status as u16;
        }
    }
}

fn write_extended(register: u16, value: u16) {
    write(REG_REGCR, REGCR_ADDRESS);
    write(REG_ADDAR, register);
    write(REG_REGCR, REGCR_DATA);
    write(REG_ADDAR, value);
}

/// PHY のリセットの解除から、MDIO を使えるまで待った後に呼ぶ。
pub fn init_phy() {
    write_extended(REG_RGMIICTL, RGMIICTL_VALUE);
    write_extended(REG_RGMIIDCTL, RGMIIDCTL_VALUE);
    write(REG_CFG1, CFG1_VALUE);
    write(REG_BMCR, BMCR_VALUE);
}

pub struct PhyStatus {
    pub link: bool,
    pub speed_mbps: u32,
    pub full_duplex: bool,
}

pub fn phy_status() -> PhyStatus {
    let physts = read(REG_PHYSTS);
    let speed = (physts >> PHYSTS_SPEED_SHIFT) as usize;
    PhyStatus {
        link: physts & PHYSTS_LINK != 0,
        speed_mbps: SPEEDS_MBPS.get(speed).copied().unwrap_or(0),
        full_duplex: physts & PHYSTS_DUPLEX != 0,
    }
}
