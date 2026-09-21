//! UART0 を、8N1 の非同期シリアルとして使う。

use crate::pac;
use core::convert::Infallible;

type Registers = &'static pac::uart0::RegisterBlock;

pub struct Uart {
    tx: UartTx,
    rx: UartRx,
}

/// 送信は、DATA への書き込みと、CTRL の送信のビットだけを使う。
pub struct UartTx {
    regs: Registers,
}

/// 受信は、DATA からの読み出しと、CTRL の受信のビットだけを使う。
pub struct UartRx {
    regs: Registers,
}

impl Uart {
    /// UART はクロックを 2 分周してから、分周比で 1 ビットの長さを作る。
    /// 前置分周は使わないため、分周比が 10 ビットに収まる速度で使う。
    pub fn new(_uart0: pac::Uart0, clk_hz: u32, baud: u32) -> Self {
        // 送信と受信は同じレジスタの別のビットを使うため、両方が同じレジスタを指しても互いに干渉しない。
        // pac::Uart0 を受け取ることで、UART0 を指すのはこの型だけになる。
        let regs: Registers = unsafe { &*pac::Uart0::ptr() };
        let divider = clk_hz / (2 * baud);
        // SVD は分周比の欄に値の範囲を持たないため、bits で書く。10 ビットに収まる速度で使う前提である。
        regs.ctrl()
            .write(|w| unsafe { w.uart_ctrl_en().set_bit().uart_ctrl_baud().bits((divider - 1) as u16) });
        Uart {
            tx: UartTx { regs },
            rx: UartRx { regs },
        }
    }

    /// 送信と受信を、別々の持ち主に渡せるように分ける。
    pub fn split(self) -> (UartTx, UartRx) {
        (self.tx, self.rx)
    }
}

impl UartTx {
    pub fn write_byte(&mut self, byte: u8) {
        while self.regs.ctrl().read().uart_ctrl_tx_nfull().bit_is_clear() {}
        self.regs.data().write(|w| unsafe { w.uart_data_rtx().bits(byte) });
    }
}

impl UartRx {
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.regs.ctrl().read().uart_ctrl_rx_nempty().bit_is_clear() {
            return None;
        }
        Some(self.regs.data().read().uart_data_rtx().bits())
    }
}

impl embedded_io::ErrorType for Uart {
    type Error = Infallible;
}

impl embedded_io::ErrorType for UartTx {
    type Error = Infallible;
}

impl embedded_io::ErrorType for UartRx {
    type Error = Infallible;
}

impl embedded_io::Write for UartTx {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Infallible> {
        for &byte in buf {
            self.write_byte(byte);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Infallible> {
        while self.regs.ctrl().read().uart_ctrl_tx_busy().bit_is_set() {}
        Ok(())
    }
}

impl embedded_io::ReadReady for UartRx {
    fn read_ready(&mut self) -> Result<bool, Infallible> {
        Ok(self.regs.ctrl().read().uart_ctrl_rx_nempty().bit_is_set())
    }
}

/// 1 バイト以上が届くまで待ち、届いている分を返す。
impl embedded_io::Read for UartRx {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Infallible> {
        let mut count = 0;
        while count < buf.len() {
            match self.read_byte() {
                Some(byte) => {
                    buf[count] = byte;
                    count += 1;
                }
                None if count > 0 => break,
                None => {}
            }
        }
        Ok(count)
    }
}

impl embedded_io::Write for Uart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Infallible> {
        self.tx.write(buf)
    }

    fn flush(&mut self) -> Result<(), Infallible> {
        self.tx.flush()
    }
}

impl embedded_io::ReadReady for Uart {
    fn read_ready(&mut self) -> Result<bool, Infallible> {
        self.rx.read_ready()
    }
}

impl embedded_io::Read for Uart {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Infallible> {
        self.rx.read(buf)
    }
}
