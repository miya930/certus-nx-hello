use control::model::NU;
use control::protocol::{Mode, SETPOINT_RANGE_C};
use control::thermistor;
use defmt::warn;
use embassy_stm32::adc::{Adc, AnyAdcChannel, SampleTime};
use embassy_stm32::peripherals::{ADC1, IWDG1};
use embassy_stm32::wdg::IndependentWatchdog;
use embassy_time::{Duration, Ticker};

use crate::heaters::Heaters;
use crate::shared;

const SAMPLE_PERIOD: Duration = Duration::from_millis(100);
// PWM の周期より十分に長く、熱の時定数より十分に短い時間で平滑化する。
const FILTER_TIME_S: f32 = 1.0;
const FILTER_ALPHA: f32 = SAMPLE_PERIOD.as_millis() as f32 / 1000.0 / FILTER_TIME_S;

// 分圧の出力インピーダンスは数 kΩ あるため、16 bit の精度を出せるよう長めに取る。
const ADC_SAMPLE_TIME: SampleTime = SampleTime::CYCLES387_5;

// 目標温度の上限でのオーバーシュートと雑音では止まらず、素子の温度の上限を大きく超える前に止まる余裕にする。
// ヒーターの近くはサーミスタより数 K 高くなるため、余裕は小さく取る。
const TEMPERATURE_MARGIN_K: f32 = 5.0;
// 断線ではとても低い温度、短絡ではとても高い温度に見えるため、同じ範囲の確認で見つかる。
const TEMPERATURE_LIMIT_C: f32 = *SETPOINT_RANGE_C.end() + TEMPERATURE_MARGIN_K;
const TEMPERATURE_MIN_C: f32 = -20.0;

pub struct Io {
    pub adc: Adc<'static, ADC1>,
    pub sensors: [AnyAdcChannel<'static, ADC1>; NU],
    pub heaters: Heaters,
    pub watchdog: IndependentWatchdog<'static, IWDG1>,
}

/// サーミスタを読み、制御器の電力をヒーターにかける。
/// 制御器より高い優先度で動かし、制御器が計算中でも過熱したら止められるようにする。
/// ウォッチドッグもここで更新し、このタスクが止まったらリセットでヒーターを切る。
#[embassy_executor::task]
pub async fn run(mut io: Io) -> ! {
    let mut filtered = [0.0; NU];
    for (t, sensor) in filtered.iter_mut().zip(io.sensors.iter_mut()) {
        *t = thermistor::temperature(io.adc.blocking_read(sensor, ADC_SAMPLE_TIME));
    }
    io.watchdog.unleash();
    let mut ticker = Ticker::every(SAMPLE_PERIOD);
    loop {
        io.watchdog.pet();
        for (t, sensor) in filtered.iter_mut().zip(io.sensors.iter_mut()) {
            let sample = thermistor::temperature(io.adc.blocking_read(sensor, ADC_SAMPLE_TIME));
            *t += FILTER_ALPHA * (sample - *t);
        }
        let fault = filtered.iter().any(|t| !(TEMPERATURE_MIN_C..=TEMPERATURE_LIMIT_C).contains(t));
        let power = shared::update(|s| {
            s.temperature = filtered;
            if fault && !s.fault {
                warn!("heaters stopped: temperature out of range {}", filtered);
            }
            s.fault = fault;
            // 異常が続く間は制御を止めたままにし、戻ったあとも利用者が改めて mode を送るまで再開しない。
            if fault {
                s.mode = Mode::Off;
            }
            s.applied_power()
        });
        io.heaters.set_power(&power);
        ticker.next().await;
    }
}
