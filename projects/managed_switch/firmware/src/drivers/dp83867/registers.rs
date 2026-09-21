//! DP83867 のレジスタの番号とビット。

pub const BMCR: u8 = 0x00;
pub const ANLPAR: u8 = 0x05;
pub const ANER: u8 = 0x06;
pub const CFG1: u8 = 0x09;
pub const STS1: u8 = 0x0A;
pub const REGCR: u8 = 0x0D;
pub const ADDAR: u8 = 0x0E;
pub const PHYSTS: u8 = 0x11;
pub const RECR: u8 = 0x15;

// 拡張レジスタは MDIO で直接指せないため、番号を ADDAR に書いて指す。
pub const RGMIICTL: u16 = 0x0032;
pub const RGMIIDCTL: u16 = 0x0086;

// REGCR のビット 15 と 14 は、ADDAR に書くものがアドレスかデータかを選ぶ。
// ビット 4 から 0 はデバイスのアドレスで、DP83867 の拡張レジスタは 0x1F にある。
pub const REGCR_ADDRESS: u16 = 0x001F;
pub const REGCR_DATA: u16 = 0x401F;

// PHYSTS のビット 15 と 14 が速度、13 が全二重、10 がリンク、8 が A と B の対の MDI-X を示す。
pub const PHYSTS_SPEED_SHIFT: u16 = 14;
pub const PHYSTS_DUPLEX: u16 = 1 << 13;
pub const PHYSTS_LINK: u16 = 1 << 10;
pub const PHYSTS_MDI_X_MODE_AB: u16 = 1 << 8;

// ANER のビット 0 は、相手が自動交渉に対応することを示す。
pub const ANER_LP_AN_ABLE: u16 = 1 << 0;

// ANLPAR は、相手が広告した PAUSE と、100BASE-TX と 10BASE-Te の全二重と半二重を示す。
pub const ANLPAR_ASM_DIR: u16 = 1 << 11;
pub const ANLPAR_PAUSE: u16 = 1 << 10;
pub const ANLPAR_TX_FD: u16 = 1 << 8;
pub const ANLPAR_TX: u16 = 1 << 7;
pub const ANLPAR_10_FD: u16 = 1 << 6;
pub const ANLPAR_10: u16 = 1 << 5;

// STS1 のビット 11 と 10 は、相手の 1000BASE-T の全二重と半二重を示す。
pub const STS1_1000BASE_T_FD: u16 = 1 << 11;
pub const STS1_1000BASE_T_HD: u16 = 1 << 10;
