//! ボードの Ethernet PHY である DP83867 を、MDIO で設定し、状態を読む。

mod registers;

use super::switch::Mdio;
use registers::*;

// RGMIICTL は、RGMII を有効にし、送信と受信のクロックをデータに対してずらす設定にする。
const RGMIICTL_VALUE: u16 = 0x00D3;

// RGMIIDCTL は、送信のクロックのずれを 4.00 ns に、受信のクロックのずれを 2.00 ns にする。
// FPGA 側ではクロックをずらさないため、ずれは PHY だけで作る。
// 100 Mbps で送信のずれを 0.25 ns ずつ変えると、2.00 ns 以下ではフレームが化けるか届かず、2.25 ns 以上で全て届いた。
// 設定できる最大の 4.00 ns にして、化ける側の際から離す。受信は全ての設定で損失がなかった。
const RGMIIDCTL_VALUE: u16 = 0x00F7;

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
    /// 1000BASE-T でなければ、使うのは A と B の対だけなので、その対の結果を返す。
    pub mdi_x: bool,
}

/// 相手が自動交渉で広告した能力。
pub struct Partner {
    pub full_1000: bool,
    pub half_1000: bool,
    pub full_100: bool,
    pub half_100: bool,
    pub full_10: bool,
    pub half_10: bool,
    pub pause: bool,
    pub asymmetric_pause: bool,
}

#[derive(Clone, Copy)]
pub struct Dp83867 {
    mdio: Mdio,
    phy_addr: u8,
}

impl Dp83867 {
    pub const fn new(mdio: Mdio, phy_addr: u8) -> Self {
        Dp83867 { mdio, phy_addr }
    }

    fn read(&self, register: u8) -> u16 {
        self.mdio.read(self.phy_addr, register)
    }

    fn write(&self, register: u8, value: u16) {
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
        // コアだけをリセットしたときも、ポートの統計と同じく起動から数える。
        self.clear_receive_errors();
    }

    pub fn status(&self) -> Status {
        let physts = self.read(PHYSTS);
        let speed = (physts >> PHYSTS_SPEED_SHIFT) as usize;
        Status {
            link: physts & PHYSTS_LINK != 0,
            speed_mbps: SPEEDS_MBPS.get(speed).copied().unwrap_or(0),
            full_duplex: physts & PHYSTS_DUPLEX != 0,
            mdi_x: physts & PHYSTS_MDI_X_MODE_AB != 0,
        }
    }

    /// 相手が自動交渉に対応しなければ None を返す。
    pub fn partner(&self) -> Option<Partner> {
        if self.read(ANER) & ANER_LP_AN_ABLE == 0 {
            return None;
        }
        let anlpar = self.read(ANLPAR);
        // CFG1 で 1000BASE-T を広告していなくても、STS1 は相手の 1000BASE-T の能力を返すことを実機で確かめた。
        let sts1 = self.read(STS1);
        Some(Partner {
            full_1000: sts1 & STS1_1000BASE_T_FD != 0,
            half_1000: sts1 & STS1_1000BASE_T_HD != 0,
            full_100: anlpar & ANLPAR_TX_FD != 0,
            half_100: anlpar & ANLPAR_TX != 0,
            full_10: anlpar & ANLPAR_10_FD != 0,
            half_10: anlpar & ANLPAR_10 != 0,
            pause: anlpar & ANLPAR_PAUSE != 0,
            asymmetric_pause: anlpar & ANLPAR_ASM_DIR != 0,
        })
    }

    /// PHY が受信で RX_ER を出した回数。65535 で止まる。
    pub fn receive_errors(&self) -> u16 {
        self.read(RECR)
    }

    /// RECR は、何を書いても 0 に戻る。
    pub fn clear_receive_errors(&self) {
        self.write(RECR, 0);
    }
}
