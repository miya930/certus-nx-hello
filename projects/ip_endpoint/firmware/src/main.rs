#![no_std]
#![no_main]

mod mailmap;

use core::ptr::{read_volatile, write_volatile};
use panic_halt as _;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::time::Instant;
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

fn uart_hex(value: u32) {
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as u8;
        uart_put(if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 });
    }
}

fn uart_decimal(value: u32) {
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut rest = value;
    loop {
        digits[count] = b'0' + (rest % 10) as u8;
        rest /= 10;
        count += 1;
        if rest == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        uart_put(digits[count]);
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    uart_puts("\nIP endpoint on NEORV32.\n");

    // ConfigBus が通っているかを確かめる。どちらのアクセスで止まるかを出し分ける。
    const MAILMAP_TX: *mut u32 = 0x9000_1800 as *mut u32;
    uart_puts("before write\n");
    unsafe { write_volatile(MAILMAP_TX, 0xA5A5_1234) };
    uart_puts("after write\n");
    let readback = unsafe { read_volatile(MAILMAP_TX as *const u32) };
    uart_puts("readback=");
    uart_hex(readback);
    uart_puts("\n");
    uart_puts("Address 169.254.111.50/16, MAC 5A:5A:00:00:00:02.\n");

    let mut device = mailmap::MailMap::new();
    let config = Config::new(EthernetAddress(MAC).into());
    let mut iface = Interface::new(config, &mut device, now());
    iface.update_ip_addrs(|addrs| {
        addrs
            .push(IpCidr::Ipv4(Ipv4Cidr::new(IP, PREFIX)))
            .expect("address list is full");
    });

    // 設定したアドレスが実際に入ったかを確かめる。
    uart_puts("iface ip=");
    match iface.ipv4_addr() {
        Some(addr) => {
            let octets = addr.octets();
            for (n, byte) in octets.iter().enumerate() {
                if n > 0 {
                    uart_put(b'.');
                }
                uart_decimal(*byte as u32);
            }
        }
        None => uart_puts("none"),
    }
    uart_puts("\n");

    // ソケットは開かない。ARP と ICMP の応答は smoltcp が IP の層で処理する。
    let mut storage: [SocketStorage; 1] = Default::default();
    let mut sockets = SocketSet::new(&mut storage[..]);

    // LED の下位 7 ビットに受け取ったフレーム数を出し、最上位を毎秒反転させて動作を示す。
    let mut heartbeat: u32 = 0;
    let mut next_beat = now() + smoltcp::time::Duration::from_secs(1);

    loop {
        iface.poll(now(), &mut device, &mut sockets);
        if now() >= next_beat {
            heartbeat ^= 1;
            next_beat = now() + smoltcp::time::Duration::from_secs(1);
            // 受信できているかを目で確かめるため、毎秒その数と mailmap のレジスタを出す。
            const MAILMAP_RX_CTRL: *const u32 = 0x9000_17FC as *const u32;
            const MAILMAP_TX_CTRL: *const u32 = 0x9000_1FFC as *const u32;
            uart_puts("rx=");
            uart_decimal(device.rx_count);
            uart_puts(" rxctrl=");
            uart_hex(unsafe { read_volatile(MAILMAP_RX_CTRL) });
            uart_puts(" txctrl=");
            uart_hex(unsafe { read_volatile(MAILMAP_TX_CTRL) });
            // GPIO 入力には RGMII のポートの状態が入る。
            const GPIO_PORT_IN: *const u32 = 0xFFFC_0000 as *const u32;
            let state = unsafe { read_volatile(GPIO_PORT_IN) };
            uart_puts(" rate=");
            uart_decimal(state & 0xFFFF);
            uart_puts(" status=");
            uart_hex((state >> 16) & 0xFF);
            uart_puts(" mdio=");
            uart_decimal((state >> 24) & 1);
            uart_puts(" prst=");
            uart_decimal((state >> 25) & 3);
            uart_puts(" len=");
            uart_decimal(device.rx_last_len);
            uart_puts(" etype=");
            uart_hex(device.rx_last_etype);
            uart_puts(" tx=");
            uart_decimal(unsafe { mailmap::TX_COUNT });
            uart_puts(" proto=");
            uart_decimal(device.rx_last_proto);
            uart_puts(" src=");
            uart_hex(device.rx_last_src_ip);
            uart_puts(" dstip=");
            uart_hex(device.rx_last_dst_ip);
            uart_puts(" ipsum=");
            uart_hex(device.rx_last_ipsum);
            uart_puts("\n");
        }
        unsafe { write_volatile(GPIO_PORT_OUT, (heartbeat << 7) | (device.rx_count & 0x7F)) };
    }
}
