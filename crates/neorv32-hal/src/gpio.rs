//! GPIO を、32 ビットの入力と出力のポートとして使う。

use crate::pac;

pub struct Gpio {
    regs: pac::Gpio,
}

impl Gpio {
    pub fn new(regs: pac::Gpio) -> Self {
        Gpio { regs }
    }

    pub fn read(&self) -> u32 {
        self.regs.port_in().read().bits()
    }

    pub fn write(&mut self, value: u32) {
        self.regs.port_out().write(|w| unsafe { w.bits(value) });
    }
}
