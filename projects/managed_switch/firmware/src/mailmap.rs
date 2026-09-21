//! SatCat5 の port_mailmap を smoltcp の Device として扱う。

use core::ptr::{read_volatile, write_volatile};
use smoltcp::phy::{self, Checksum, ChecksumCapabilities, DeviceCapabilities, Medium};
use smoltcp::time::Instant;

/// ConfigBus のデバイス 1 が並ぶ位置。デバイス番号を 12 ビット左に寄せた先になる。
const BASE: usize = 0x9000_1000;

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

/// ConfigBus を Wishbone 経由でつないでいるため、バイト単位の書き込みができない。
/// フレームはワード単位で読み書きし、バイトへの詰め替えは CPU 側で行う。
fn read_reg(index: usize) -> u32 {
    unsafe { read_volatile((BASE + index * 4) as *const u32) }
}

fn write_reg(index: usize, value: u32) {
    unsafe { write_volatile((BASE + index * 4) as *mut u32, value) };
}

pub struct MailMap {
    rx_buffer: [u8; BUFFER_BYTES],
    tx_buffer: [u8; BUFFER_BYTES],
    /// 受け取ったフレームの数。動作の確認に使う。
    pub rx_count: u32,
}

impl MailMap {
    pub const fn new() -> Self {
        Self {
            rx_buffer: [0; BUFFER_BYTES],
            tx_buffer: [0; BUFFER_BYTES],
            rx_count: 0,
        }
    }

    fn rx_length(&self) -> usize {
        read_reg(REG_RX_CTRL) as usize
    }

    /// 受信したフレームをワード単位で読み出し、読み終えたことを知らせて次へ進める。
    fn take_frame(&mut self, length: usize) {
        for offset in (0..length).step_by(4) {
            let word = read_reg(REG_RX_DATA + offset / 4).to_le_bytes();
            let remain = (length - offset).min(4);
            self.rx_buffer[offset..offset + remain].copy_from_slice(&word[..remain]);
        }
        write_reg(REG_RX_CTRL, 0);
        self.rx_count = self.rx_count.wrapping_add(1);
    }

    fn transmit_busy(&self) -> bool {
        read_reg(REG_TX_CTRL) != 0
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
        let length = self.rx_length();
        if length == 0 || length > BUFFER_BYTES || self.transmit_busy() {
            return None;
        }
        self.take_frame(length);
        let (rx_buffer, tx_buffer) = (&self.rx_buffer[..length], &mut self.tx_buffer);
        Some((RxToken { buffer: rx_buffer }, TxToken { buffer: tx_buffer }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        if self.transmit_busy() {
            return None;
        }
        Some(TxToken {
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
    buffer: &'a mut [u8],
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, length: usize, f: F) -> R {
        let result = f(&mut self.buffer[..length]);
        // 送信中はバッファへの書き込みが無視されるため、空くまで待つ。
        while read_reg(REG_TX_CTRL) != 0 {}
        // TxToken は送信バッファだけを借りているため、ここでレジスタへ直接書き出す。
        for offset in (0..length).step_by(4) {
            let mut word = [0u8; 4];
            let remain = (length - offset).min(4);
            word[..remain].copy_from_slice(&self.buffer[offset..offset + remain]);
            write_reg(REG_TX_DATA + offset / 4, u32::from_le_bytes(word));
        }
        write_reg(REG_TX_CTRL, length as u32);
        result
    }
}
