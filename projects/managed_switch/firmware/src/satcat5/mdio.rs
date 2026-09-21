//! SatCat5 の cfgbus_mdio で、MDIO の読み書きをする。

use super::Device;

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

#[derive(Clone, Copy)]
pub struct Mdio {
    device: Device,
}

impl Mdio {
    pub const fn new(device: Device) -> Self {
        Mdio { device }
    }

    fn command(&self, op: u32, phy: u32, register: u32, value: u16) {
        while self.device.read(REG_MDIO) & STATUS_FULL != 0 {}
        self.device.write(REG_MDIO, op | phy << PHY_SHIFT | register << REG_SHIFT | value as u32);
    }

    pub fn write(&self, phy: u32, register: u32, value: u16) {
        self.command(OP_WRITE, phy, register, value);
    }

    pub fn read(&self, phy: u32, register: u32) -> u16 {
        self.command(OP_READ, phy, register, 0);
        loop {
            let status = self.device.read(REG_MDIO);
            if status & STATUS_VALID != 0 {
                return status as u16;
            }
        }
    }
}
