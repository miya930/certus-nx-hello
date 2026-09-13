#![no_std]
#![no_main]

mod controller;
mod heaters;
mod network;
mod sampling;
mod shared;

use cortex_m_rt::entry;
use defmt::{info, unwrap};
use embassy_executor::{Executor, InterruptExecutor};
use embassy_stm32::adc::{Adc, AdcChannel, AdcConfig, Averaging, Resolution};
use embassy_stm32::eth::{Ethernet, PacketQueue};
use embassy_stm32::interrupt::{InterruptExt, Priority};
use embassy_stm32::rng::Rng;
use embassy_stm32::wdg::IndependentWatchdog;
use embassy_stm32::{bind_interrupts, eth, interrupt, peripherals, rng};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::heaters::{Heaters, Pins};

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

// サーミスタの読み取りの周期より十分に長くし、パニックで止まったときはリセットでヒーターを切る。
const WATCHDOG_TIMEOUT_US: u32 = 1_000_000;

// ローカルに管理するアドレスのビットを立てた MAC アドレス。
const MAC_ADDRESS: [u8; 6] = [0x02, 0x00, 0x00, 0x48, 0x37, 0x53];

// 入出力のタスクは割り込みの executor で動かし、スレッドモードで計算中の MPC より先に動けるようにする。
// UART4 は使っていないので、その割り込みベクタを executor の起動に借りる。
static EXECUTOR_IO: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_CONTROL: StaticCell<Executor> = StaticCell::new();

#[interrupt]
unsafe fn UART4() {
    unsafe { EXECUTOR_IO.on_interrupt() }
}

fn clock_config() -> embassy_stm32::Config {
    use embassy_stm32::rcc::*;
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = Some(HSIPrescaler::DIV1);
    config.rcc.csi = true;
    config.rcc.hsi48 = Some(Default::default());
    // HSI 64 MHz / 4 × 50 = 800 MHz を 2 で割って 400 MHz にする。
    config.rcc.pll1 = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV4,
        mul: PllMul::MUL50,
        divp: Some(PllDiv::DIV2),
        divq: None,
        divr: None,
    });
    // ADC には 800 MHz を 8 で割った 100 MHz を与え、ADC のドライバが上限の周波数以下に分周する。
    config.rcc.pll2 = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV4,
        mul: PllMul::MUL50,
        divp: Some(PllDiv::DIV8),
        divq: None,
        divr: None,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV2;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV2;
    config.rcc.apb3_pre = APBPrescaler::DIV2;
    config.rcc.apb4_pre = APBPrescaler::DIV2;
    config.rcc.voltage_scale = VoltageScale::Scale1;
    config.rcc.mux.adcsel = mux::Adcsel::PLL2_P;
    config
}

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init(clock_config());
    info!("heater controller started");

    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    rng.fill_bytes(&mut seed);

    // RMII のピンは NUCLEO-H753ZI の LAN8742A につながっているもの。
    static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
    let device = Ethernet::new(
        PACKETS.init(PacketQueue::<4, 4>::new()),
        p.ETH,
        Irqs,
        p.PA1,
        p.PA7,
        p.PC4,
        p.PC5,
        p.PG13,
        p.PB13,
        p.PG11,
        MAC_ADDRESS,
        p.ETH_SMA,
        p.PA2,
        p.PC1,
    );

    // サーミスタとヒーターのピンは、ヒーターの基板が決まるまでの仮の割り当て。
    // Ethernet、LED、ST-LINK の仮想 COM ポートに使われているピンを避けている。
    let adc_config = AdcConfig { resolution: Some(Resolution::BITS16), averaging: Some(Averaging::Samples16) };
    let io = sampling::Io {
        adc: Adc::new_with_config(p.ADC1, adc_config),
        sensors: [
            p.PA3.degrade_adc(),
            p.PC0.degrade_adc(),
            p.PB1.degrade_adc(),
            p.PA6.degrade_adc(),
            p.PA5.degrade_adc(),
            p.PA4.degrade_adc(),
            p.PA0.degrade_adc(),
            p.PF11.degrade_adc(),
            p.PF12.degrade_adc(),
        ],
        heaters: Heaters::new(Pins {
            tim1: p.TIM1,
            pe9: p.PE9,
            pe11: p.PE11,
            pe13: p.PE13,
            pe14: p.PE14,
            tim4: p.TIM4,
            pd12: p.PD12,
            pd13: p.PD13,
            pd14: p.PD14,
            pd15: p.PD15,
            tim3: p.TIM3,
            pc6: p.PC6,
        }),
        watchdog: IndependentWatchdog::new(p.IWDG1, WATCHDOG_TIMEOUT_US),
    };

    interrupt::UART4.set_priority(Priority::P6);
    let spawner = EXECUTOR_IO.start(interrupt::UART4);
    spawner.spawn(unwrap!(sampling::run(io)));
    spawner.spawn(unwrap!(network::run(device, u64::from_le_bytes(seed))));

    let executor = EXECUTOR_CONTROL.init(Executor::new());
    executor.run(|spawner| spawner.spawn(unwrap!(controller::run())))
}
