//! SatCat5 の port_mailmap を smoltcp の Device として扱う。

use super::Device;
use smoltcp::phy::{self, Checksum, ChecksumCapabilities, DeviceCapabilities, Medium};
use smoltcp::time::Instant;

/// 受信したフレームの中身が並ぶ先頭のレジスタ。
const REG_RX_DATA: usize = 0;
/// 読むと受信したフレームの長さ、書くと次のフレームへ進む。
const REG_RX_CTRL: usize = 511;
/// 送信するフレームを置く先頭のレジスタ。
const REG_TX_DATA: usize = 512;
/// 書くと送信する長さ、読むと送信中かどうかを返す。
const REG_TX_CTRL: usize = 1023;

/// 受信と送信それぞれに割り当てられたレジスタの数から決まる上限。
const MTU: usize = 1514;
const BUFFER_BYTES: usize = 1600;
const WORD_BYTES: usize = 4;

/// port_mailmap のレジスタ。ConfigBus への橋渡しはバイト単位の書き込みを持たないため、フレームはワード単位で読み書きする。
#[derive(Clone, Copy)]
pub struct MailMapPort {
    device: Device,
}

impl MailMapPort {
    pub const fn new(device: Device) -> Self {
        MailMapPort { device }
    }

    fn rx_length(&self) -> usize {
        self.device.read(REG_RX_CTRL) as usize
    }

    /// 受信したフレームを読み出し、読み終えたことを知らせて次へ進める。
    fn take_frame(&self, buffer: &mut [u8]) {
        for (index, chunk) in buffer.chunks_mut(WORD_BYTES).enumerate() {
            let word = self.device.read(REG_RX_DATA + index).to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
        self.device.write(REG_RX_CTRL, 0);
    }

    fn transmit_busy(&self) -> bool {
        self.device.read(REG_TX_CTRL) != 0
    }

    /// 送信中はバッファへの書き込みが無視されるため、空くまで待ってから書く。
    fn send_frame(&self, frame: &[u8]) {
        while self.transmit_busy() {}
        for (index, chunk) in frame.chunks(WORD_BYTES).enumerate() {
            let mut word = [0u8; WORD_BYTES];
            word[..chunk.len()].copy_from_slice(chunk);
            self.device.write(REG_TX_DATA + index, u32::from_le_bytes(word));
        }
        self.device.write(REG_TX_CTRL, frame.len() as u32);
    }
}

pub struct MailMap {
    port: MailMapPort,
    rx_buffer: [u8; BUFFER_BYTES],
    tx_buffer: [u8; BUFFER_BYTES],
    /// 受け取ったフレームの数。動作の確認に使う。
    pub rx_count: u32,
}

impl MailMap {
    pub const fn new(port: MailMapPort) -> Self {
        Self {
            port,
            rx_buffer: [0; BUFFER_BYTES],
            tx_buffer: [0; BUFFER_BYTES],
            rx_count: 0,
        }
    }
}

impl phy::Device for MailMap {
    type RxToken<'a> = RxToken<'a>;
    type TxToken<'a> = TxToken<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = MTU;
        // FCS は port_mailmap が付けて外すため、上位では扱わない。
        let mut checksum = ChecksumCapabilities::default();
        checksum.ipv4 = Checksum::Both;
        checksum.icmpv4 = Checksum::Both;
        caps.checksum = checksum;
        caps
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let length = self.port.rx_length();
        if length == 0 || length > BUFFER_BYTES || self.port.transmit_busy() {
            return None;
        }
        self.port.take_frame(&mut self.rx_buffer[..length]);
        self.rx_count = self.rx_count.wrapping_add(1);
        let port = self.port;
        let (rx_buffer, tx_buffer) = (&self.rx_buffer[..length], &mut self.tx_buffer);
        Some((RxToken { buffer: rx_buffer }, TxToken { port, buffer: tx_buffer }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        if self.port.transmit_busy() {
            return None;
        }
        Some(TxToken {
            port: self.port,
            buffer: &mut self.tx_buffer,
        })
    }
}

pub struct RxToken<'a> {
    buffer: &'a [u8],
}

impl<'a> phy::RxToken for RxToken<'a> {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(self.buffer)
    }
}

pub struct TxToken<'a> {
    port: MailMapPort,
    buffer: &'a mut [u8],
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, length: usize, f: F) -> R {
        let result = f(&mut self.buffer[..length]);
        self.port.send_frame(&self.buffer[..length]);
        result
    }
}
