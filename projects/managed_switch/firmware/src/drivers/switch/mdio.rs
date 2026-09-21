//! スイッチの MDIO で、PHY のレジスタを読み書きする。

use satcat5_pac::mdio;

/// 操作は FIFO に積まれ、読み出した値も FIFO に積まれて、レジスタを読むたびに 1 つずつ取り出される。
#[derive(Clone, Copy)]
pub struct Mdio {
    regs: *const mdio::RegisterBlock,
}

impl Mdio {
    pub(super) const fn new(regs: *const mdio::RegisterBlock) -> Self {
        Mdio { regs }
    }

    fn register(&self) -> &mdio::Mdio {
        unsafe { &*self.regs }.mdio()
    }

    fn command(&self, operation: impl FnOnce(&mut mdio::mdio::W) -> &mut mdio::mdio::W) {
        let register = self.register();
        while register.read().full().bit_is_set() {}
        register.write(operation);
    }

    pub fn write(&self, phy: u8, register: u8, value: u16) {
        self.command(|w| w.op().write().phy().set(phy).reg().set(register).data().set(value));
    }

    pub fn read(&self, phy: u8, register: u8) -> u16 {
        self.command(|w| w.op().read().phy().set(phy).reg().set(register));
        loop {
            let status = self.register().read();
            if status.valid().bit_is_set() {
                return status.data().bits();
            }
        }
    }
}
