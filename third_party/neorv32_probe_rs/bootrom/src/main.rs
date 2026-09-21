//! NEORV32 の起動 ROM。
//! 命令メモリが空なら、SPI Flash に書いたファームウェアを命令メモリにコピーしてから、命令メモリの先頭へ飛ぶ。
//! JTAG から書き込んだあとのリセットでは、命令メモリが空でないため、コピーせずにそのまま飛ぶ。

#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use mt25q::Mt25q;
use neorv32_hal::{pac, spi::Spi};

/// firmware image を置く SPI Flash の位置。ビットストリームと設定のあいだにある。
/// tools/firmware_flash と、各プロジェクトの neorv32.yaml の Flash の範囲に合わせる。
const IMAGE_OFFSET: u32 = 0x00F0_0000;
/// firmware image の先頭は、この 4 文字と、続く中身のバイト数の 4 バイトである。
const IMAGE_MAGIC: [u8; 4] = *b"IMEM";
const HEADER_BYTES: usize = 8;

/// この基板の Flash の線は、1 MHz では READ ID が半分ほど失敗し、100 kHz では失敗しなかった。
const FLASH_SCK_HZ: u32 = 100_000;
/// Flash は、SPI の 0 番のチップセレクトにつながる。
const FLASH_CS: u8 = 0;

/// 一度の READ で読む大きさ。スタックに置くため、小さくしておく。
const CHUNK_BYTES: usize = 256;
const WORD_BYTES: usize = 4;

global_asm!(".section .text.start", ".global _start", "_start:", "la sp, _stack_top", "j boot");

/// 命令メモリはアドレス 0 から始まる。
/// Rust ではアドレス 0 を指すポインタを読み書きできないため、命令メモリへのアクセスはアセンブリで行う。
fn imem_read(address: usize) -> u32 {
    let word;
    unsafe { asm!("lw {word}, 0({address})", word = out(reg) word, address = in(reg) address) };
    word
}

fn imem_write(address: usize, word: u32) {
    unsafe { asm!("sw {word}, 0({address})", word = in(reg) word, address = in(reg) address) };
}

/// Flash に正しい firmware image があれば、命令メモリにコピーする。なければ何もしない。
fn load_from_flash() {
    let peripherals = unsafe { pac::Peripherals::steal() };
    let sysinfo = &peripherals.sysinfo;
    // 起動 ROM は複数のプロジェクトで共有するため、SPI を持たない設計でも動くようにする。
    if sysinfo.soc().read().sysinfo_soc_io_spi().bit_is_clear() {
        return;
    }
    let clk_hz = sysinfo.clk().read().bits();
    let imem_bytes = 1usize << sysinfo.misc().read().sysinfo_misc_imem().bits();
    let mut flash = Mt25q::new(Spi::new(peripherals.spi, clk_hz, FLASH_SCK_HZ, FLASH_CS));

    let mut header = [0; HEADER_BYTES];
    let Ok(()) = flash.read(IMAGE_OFFSET, &mut header);
    let (magic, length) = header.split_at(IMAGE_MAGIC.len());
    let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
    if magic != IMAGE_MAGIC || length == 0 || length > imem_bytes {
        return;
    }

    let mut chunk = [0; CHUNK_BYTES];
    for start in (0..length).step_by(CHUNK_BYTES) {
        let bytes = &mut chunk[..CHUNK_BYTES.min(length - start)];
        let Ok(()) = flash.read(IMAGE_OFFSET + (HEADER_BYTES + start) as u32, bytes);
        // firmware image の長さはワードの倍数にそろえてあるため、ワードごとに書く。
        for (index, word) in bytes.chunks_exact(WORD_BYTES).enumerate() {
            imem_write(start + index * WORD_BYTES, u32::from_le_bytes(word.try_into().unwrap()));
        }
    }
}

#[no_mangle]
extern "C" fn boot() -> ! {
    // 0 は不正命令なので、正しいファームウェアの先頭が 0 になることはない。
    if imem_read(0) == 0 {
        load_from_flash();
    }
    // Flash に firmware image がなければ、JTAG から書き込まれるまで待つ。
    while imem_read(0) == 0 {}
    unsafe { asm!("fence.i", "jr zero", options(noreturn)) }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
