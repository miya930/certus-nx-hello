//! DP83867 のレジスタの番号とビット。

pub const BMCR: u32 = 0x00;
pub const CFG1: u32 = 0x09;
pub const REGCR: u32 = 0x0D;
pub const ADDAR: u32 = 0x0E;
pub const PHYSTS: u32 = 0x11;

// 拡張レジスタは MDIO で直接指せないため、番号を ADDAR に書いて指す。
pub const RGMIICTL: u16 = 0x0032;
pub const RGMIIDCTL: u16 = 0x0086;

// REGCR のビット 15 と 14 は、ADDAR に書くものがアドレスかデータかを選ぶ。
// ビット 4 から 0 はデバイスのアドレスで、DP83867 の拡張レジスタは 0x1F にある。
pub const REGCR_ADDRESS: u16 = 0x001F;
pub const REGCR_DATA: u16 = 0x401F;

// PHYSTS のビット 15 と 14 が速度、13 が全二重、10 がリンクを示す。
pub const PHYSTS_SPEED_SHIFT: u16 = 14;
pub const PHYSTS_DUPLEX: u16 = 1 << 13;
pub const PHYSTS_LINK: u16 = 1 << 10;
