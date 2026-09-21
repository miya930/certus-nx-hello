//! SPI を、チップセレクトを含む embedded-hal の SpiDevice として使う。
//! クロックはモード 0 で、SCK はコアのクロックを前置分周と分周で割って作る。

use crate::pac;
use core::convert::Infallible;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

/// 前置分周の選択値 0 から 7 に対応する分周。
const PRESCALERS: [u32; 8] = [2, 4, 8, 64, 128, 1024, 2048, 4096];
/// 分周は 4 ビットで、1 から 16 までを選べる。
const DIVIDERS: u32 = 16;

// DATA のビット 31 を立てて書くと、チップセレクトの操作になる。
// ビット 3 が選ぶかどうかで、ビット 2 から 0 がチップセレクトの番号になる。
const DATA_CMD: u32 = 1 << 31;
const DATA_CS_ENABLE: u32 = 1 << 3;

pub struct Spi {
    regs: pac::Spi,
    chip_select: u8,
}

impl Spi {
    /// SCK = クロック ÷ (2 × 前置分周 × 分周) のうち、max_sck_hz を超えない一番速いものを選ぶ。
    pub fn new(regs: pac::Spi, clk_hz: u32, max_sck_hz: u32, chip_select: u8) -> Self {
        let (prescaler, divider) = (0..PRESCALERS.len() as u8)
            .flat_map(|p| (0..DIVIDERS).map(move |d| (p, d)))
            .find(|&(p, d)| clk_hz / (2 * PRESCALERS[p as usize] * (d + 1)) <= max_sck_hz)
            .unwrap_or((PRESCALERS.len() as u8 - 1, DIVIDERS - 1));
        // 選択値と分周は、どちらも上の表と範囲から選ぶため、欄の幅に収まる。
        regs.ctrl().write(|w| unsafe {
            w.spi_ctrl_en().set_bit().spi_ctrl_prsc().bits(prescaler).spi_ctrl_cdiv().bits(divider as u8)
        });
        Spi { regs, chip_select }
    }

    fn wait_idle(&self) {
        while self.regs.ctrl().read().spi_ctrl_busy().bit_is_set() {}
    }

    fn command(&mut self, word: u32) {
        self.regs.data().write(|w| unsafe { w.bits(word) });
        self.wait_idle();
    }

    fn transfer_byte(&mut self, byte: u8) -> u8 {
        self.regs.data().write(|w| unsafe { w.spi_data().bits(byte) });
        self.wait_idle();
        self.regs.data().read().spi_data().bits()
    }
}

impl ErrorType for Spi {
    type Error = Infallible;
}

impl SpiDevice for Spi {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Infallible> {
        self.command(DATA_CMD | DATA_CS_ENABLE | self.chip_select as u32);
        for operation in operations {
            match operation {
                Operation::Read(buf) => buf.iter_mut().for_each(|byte| *byte = self.transfer_byte(0)),
                Operation::Write(buf) => buf.iter().for_each(|&byte| {
                    self.transfer_byte(byte);
                }),
                Operation::Transfer(read, write) => {
                    for (index, slot) in read.iter_mut().enumerate() {
                        *slot = self.transfer_byte(write.get(index).copied().unwrap_or(0));
                    }
                }
                Operation::TransferInPlace(buf) => buf.iter_mut().for_each(|byte| *byte = self.transfer_byte(*byte)),
                Operation::DelayNs(_) => {}
            }
        }
        self.command(DATA_CMD);
        Ok(())
    }
}
