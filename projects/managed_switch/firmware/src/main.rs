#![no_std]
#![no_main]

mod mailmap;

use core::ptr::{read_volatile, write_volatile};
use panic_halt as _;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::{Duration, Instant};
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr};

// NEORV32 のメモリマップ。
const GPIO_PORT_OUT: *mut u32 = 0xFFFC_0004 as *mut u32;
const UART0_CTRL: *mut u32 = 0xFFF5_0000 as *mut u32;
const UART0_DATA: *mut u32 = 0xFFF5_0004 as *mut u32;
const CLINT_MTIME_LO: *mut u32 = 0xFFF4_BFF8 as *mut u32;
const CLINT_MTIME_HI: *mut u32 = 0xFFF4_BFFC as *mut u32;

// CTRL のビット 19 は、送信 FIFO に空きがあることを示す。
const UART_CTRL_TX_NFULL: u32 = 1 << 19;

const CLK_HZ: u32 = 25_000_000;

// ローカル管理のアドレスを使う。
// PC 側が DHCP のアドレスを取れずに使うリンクローカルの範囲に合わせてあり、直結でそのまま試せる。
const MAC: [u8; 6] = [0x5A, 0x5A, 0x00, 0x00, 0x00, 0x02];
const IP: Ipv4Address = Ipv4Address::new(169, 254, 111, 50);
const PREFIX: u8 = 16;

/// ブートローダが設定した速度をそのまま使うため、CTRL は書き換えない。
fn uart_put(byte: u8) {
    while unsafe { read_volatile(UART0_CTRL) } & UART_CTRL_TX_NFULL == 0 {}
    unsafe { write_volatile(UART0_DATA, byte as u32) };
}

fn uart_puts(text: &str) {
    for byte in text.bytes() {
        if byte == b'\n' {
            uart_put(b'\r');
        }
        uart_put(byte);
    }
}

/// マシンタイマは 64 ビットで、下位を読む間に上位が繰り上がることがある。
/// 上位が変わらなかったときの組み合わせだけを使う。
fn mtime() -> u64 {
    loop {
        let high = unsafe { read_volatile(CLINT_MTIME_HI) };
        let low = unsafe { read_volatile(CLINT_MTIME_LO) };
        if high == unsafe { read_volatile(CLINT_MTIME_HI) } {
            return ((high as u64) << 32) | low as u64;
        }
    }
}

/// タイマはシステムクロックで進むため、経過時間をクロック周波数から求める。
fn now() -> Instant {
    Instant::from_millis((mtime() / (CLK_HZ as u64 / 1000)) as i64)
}

#[riscv_rt::entry]
fn main() -> ! {
    uart_puts("\nIP endpoint on NEORV32.\n");
    uart_puts("Address 169.254.111.50/16, MAC 5A:5A:00:00:00:02.\n");

    let mut device = mailmap::MailMap::new();
    let config = Config::new(EthernetAddress(MAC).into());
    let mut iface = Interface::new(config, &mut device, now());
    iface.update_ip_addrs(|addrs| {
        addrs
            .push(IpCidr::Ipv4(Ipv4Cidr::new(IP, PREFIX)))
            .expect("address list is full");
    });

    // ソケットは開かない。ARP と ICMP の応答は smoltcp が IP の層で処理する。
    let mut storage: [SocketStorage; 1] = Default::default();
    let mut sockets = SocketSet::new(&mut storage[..]);

    // LED の下位 7 ビットに受け取ったフレーム数を出し、最上位を毎秒反転させて動作を示す。
    let mut heartbeat: u32 = 0;
    let mut next_beat = now() + Duration::from_secs(1);

    loop {
        iface.poll(now(), &mut device, &mut sockets);
        if now() >= next_beat {
            heartbeat ^= 1;
            next_beat = now() + Duration::from_secs(1);
            let mismatch = unsafe { mailmap::TX_MISMATCH };
            if mismatch != 0 {
                uart_puts("tx buffer mismatch\n");
            }
            let _ = mismatch;
        }
        unsafe { write_volatile(GPIO_PORT_OUT, (heartbeat << 7) | (device.rx_count & 0x7F)) };
    }
}
