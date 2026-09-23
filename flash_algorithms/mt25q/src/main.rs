//! probe-rs が NEORV32 の SPI で MT25QU128 に書き込むためのプログラム。
//! probe-rs はこれをデータメモリに置いて呼び、firmware image を Flash に書く。

#![no_std]
#![no_main]

use flash_algorithm::*;
use mt25q::Mt25q;
use neorv32_hal::{pac, spi::Spi};

/// probe-rs には、SPI Flash のアドレスをそのまま見せる。
/// この範囲は命令メモリとデータメモリの外にあり、コアが読み書きすることはないため、probe-rs のメモリの定義と重ならない。
/// probe-rs に見せるのは、firmware image を置く範囲だけにする。
/// ビットストリームと設定を消さないためである。範囲は third_party/neorv32_probe_rs/bootrom の IMAGE_OFFSET に合わせる。
const IMAGE_OFFSET: u32 = 0x00F0_0000;
const IMAGE_AREA_BYTES: u32 = 0x0002_0000;

/// この基板の Flash の線は、1 MHz では READ ID が半分ほど失敗し、100 kHz では失敗しなかった。
const FLASH_SCK_HZ: u32 = 100_000;
/// Flash は、SPI の 0 番のチップセレクトにつながる。
const FLASH_CS: u8 = 0;

struct Algorithm {
    flash: Mt25q<Spi>,
}

// 100 kHz では 1 ページの転送に約 21 ms かかるため、書き込みの時間切れは余裕を持たせる。
algorithm!(Algorithm, {
    device_name: "MT25QU128",
    device_type: DeviceType::ExtSpi,
    flash_address: IMAGE_OFFSET,
    flash_size: IMAGE_AREA_BYTES,
    page_size: mt25q::PAGE_BYTES as u32,
    empty_value: 0xFF,
    program_time_out: 1000,
    erase_time_out: 2000,
    sectors: [{
        size: mt25q::SUBSECTOR_BYTES,
        address: 0x0,
    }]
});

impl FlashAlgorithm for Algorithm {
    fn new(_address: u32, _clock: u32, _function: Function) -> Result<Self, ErrorCode> {
        let peripherals = unsafe { pac::Peripherals::steal() };
        let clk_hz = peripherals.sysinfo.clk().read().bits();
        Ok(Algorithm { flash: Mt25q::new(Spi::new(peripherals.spi, clk_hz, FLASH_SCK_HZ, FLASH_CS)) })
    }

    fn erase_sector(&mut self, address: u32) -> Result<(), ErrorCode> {
        let Ok(()) = self.flash.erase_subsector(address);
        Ok(())
    }

    fn program_page(&mut self, address: u32, data: &[u8]) -> Result<(), ErrorCode> {
        let Ok(()) = self.flash.program(address, data);
        Ok(())
    }

    fn read_flash(&mut self, address: u32, data: &mut [u8]) -> Result<(), ErrorCode> {
        let Ok(()) = self.flash.read(address, data);
        Ok(())
    }
}
