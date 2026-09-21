//! スイッチの CPU のポート (port_mailmap) を、smoltcp の Device として扱う。

use satcat5_pac::port_mailmap;
use smoltcp::phy::{self, Checksum, ChecksumCapabilities, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetFrame, EthernetRepr};

/// 受信と送信それぞれに割り当てられたレジスタの数から決まる上限。
const MTU: usize = 1514;
const BUFFER_BYTES: usize = 1600;
const WORD_BYTES: usize = 4;

/// port_mailmap のレジスタ。ConfigBus へのブリッジはバイト単位の書き込みを持たないため、フレームはワード単位で読み書きする。
#[derive(Clone, Copy)]
struct Mailmap {
    regs: *const port_mailmap::RegisterBlock,
}

impl Mailmap {
    fn regs(&self) -> &port_mailmap::RegisterBlock {
        unsafe { &*self.regs }
    }

    fn rx_length(&self) -> usize {
        self.regs().rx_ctrl().read().bits() as usize
    }

    /// 受信したフレームを読み出し、読み終えたことを知らせて次へ進める。
    fn take_frame(&self, buffer: &mut [u8]) {
        for (index, chunk) in buffer.chunks_mut(WORD_BYTES).enumerate() {
            let word = self.regs().rx_data(index).read().bits().to_le_bytes();
            chunk.copy_from_slice(&word[..chunk.len()]);
        }
        self.regs().rx_ctrl().write(|w| w.set(0));
    }

    fn transmit_busy(&self) -> bool {
        self.regs().tx_ctrl().read().bits() != 0
    }

    /// 送信中はバッファへの書き込みが無視されるため、空くまで待ってから書く。
    fn send_frame(&self, frame: &[u8]) {
        while self.transmit_busy() {}
        for (index, chunk) in frame.chunks(WORD_BYTES).enumerate() {
            let mut word = [0u8; WORD_BYTES];
            word[..chunk.len()].copy_from_slice(chunk);
            self.regs().tx_data(index).write(|w| w.set(u32::from_le_bytes(word)));
        }
        self.regs().tx_ctrl().write(|w| w.set(frame.len() as u32));
    }
}

pub struct CpuPort {
    mailmap: Mailmap,
    rx_buffer: [u8; BUFFER_BYTES],
    tx_buffer: [u8; BUFFER_BYTES],
    /// 受け取ったフレームの数。動作の確認に使う。
    pub rx_count: u32,
}

impl CpuPort {
    pub(super) const fn new(regs: *const port_mailmap::RegisterBlock) -> Self {
        CpuPort { mailmap: Mailmap { regs }, rx_buffer: [0; BUFFER_BYTES], tx_buffer: [0; BUFFER_BYTES], rx_count: 0 }
    }
}

impl phy::Device for CpuPort {
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
        let length = self.mailmap.rx_length();
        if length == 0 || length > BUFFER_BYTES || self.mailmap.transmit_busy() {
            return None;
        }
        self.mailmap.take_frame(&mut self.rx_buffer[..length]);
        log_frame("rx", &self.rx_buffer[..length]);
        self.rx_count = self.rx_count.wrapping_add(1);
        let mailmap = self.mailmap;
        let (rx_buffer, tx_buffer) = (&self.rx_buffer[..length], &mut self.tx_buffer);
        Some((RxToken { buffer: rx_buffer }, TxToken { mailmap, buffer: tx_buffer }))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        if self.mailmap.transmit_busy() {
            return None;
        }
        Some(TxToken { mailmap: self.mailmap, buffer: &mut self.tx_buffer })
    }
}

/// フレームの長さと Ethernet のヘッダを debug のログに出す。
fn log_frame(direction: &str, frame: &[u8]) {
    if let Ok(header) = EthernetFrame::new_checked(frame).and_then(|frame| EthernetRepr::parse(&frame)) {
        defmt::debug!("{=str} {=usize} bytes, {}", direction, frame.len(), header);
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
    mailmap: Mailmap,
    buffer: &'a mut [u8],
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, length: usize, f: F) -> R {
        let result = f(&mut self.buffer[..length]);
        log_frame("tx", &self.buffer[..length]);
        self.mailmap.send_frame(&self.buffer[..length]);
        result
    }
}
