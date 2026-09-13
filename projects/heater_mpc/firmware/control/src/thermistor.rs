use crate::model::{ADC_FULL_SCALE, THERMISTOR_BETA_K, THERMISTOR_PULLUP_OHM, THERMISTOR_R25_OHM};

const KELVIN_OFFSET: f32 = 273.15;
const T25_K: f32 = 25.0 + KELVIN_OFFSET;

/// 電源側に固定抵抗、GND 側に NTC を置いた分圧を ADC で読んだ値から、温度 [°C] を返す。
/// 断線と短絡では、それぞれ -273.15 °C と非常に高い温度になり、範囲の確認で見分けられる。
pub fn temperature(code: u16) -> f32 {
    let ratio = f32::from(code) / ADC_FULL_SCALE;
    let resistance = THERMISTOR_PULLUP_OHM * ratio / (1.0 - ratio);
    // B 定数の式 1/T = 1/T25 + ln(R/R25)/B
    let inverse = 1.0 / T25_K + libm::logf(resistance / THERMISTOR_R25_OHM) / THERMISTOR_BETA_K;
    1.0 / inverse - KELVIN_OFFSET
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_at(celsius: f32) -> u16 {
        let kelvin = celsius + KELVIN_OFFSET;
        let resistance = THERMISTOR_R25_OHM * libm::expf(THERMISTOR_BETA_K * (1.0 / kelvin - 1.0 / T25_K));
        (ADC_FULL_SCALE * resistance / (resistance + THERMISTOR_PULLUP_OHM)).round() as u16
    }

    #[test]
    fn follows_beta_equation() {
        for celsius in [0.0, 25.0, 40.0, 60.0, 90.0] {
            let measured = temperature(code_at(celsius));
            assert!((measured - celsius).abs() < 0.05, "{celsius} -> {measured}");
        }
    }

    #[test]
    fn open_and_short_are_out_of_range() {
        assert!(temperature(u16::MAX) < -200.0);
        assert!(temperature(0) < -200.0);
        assert!(temperature(1) > 1000.0);
    }
}
