use control::model::{NU, POWER_MAX_W};
use embassy_stm32::Peri;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PC6, PD12, PD13, PD14, PD15, PE9, PE11, PE13, PE14, TIM1, TIM3, TIM4};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::GeneralInstance4Channel;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm, SimplePwmChannel, SimplePwmChannels};

// ヒーターの基板が決まったら、電源電圧と抵抗値を実物に合わせる。
const SUPPLY_V: f32 = 12.0;
const HEATER_RESISTANCE_OHM: f32 = 150.0;
const FULL_POWER_W: f32 = SUPPLY_V * SUPPLY_V / HEATER_RESISTANCE_OHM;
const _: () = assert!(FULL_POWER_W >= POWER_MAX_W, "PWM cannot reach the controller's maximum power");

// ハイサイドスイッチの切り替えの損失を抑えるため、熱の時定数より十分に短い範囲で低い周波数にする。
const PWM_FREQUENCY: Hertz = Hertz(200);

/// ヒーターの番号は、MPC のモデルと同じく格子の行優先の順番。
/// リセット中と異常で停止したあとはピンがハイインピーダンスになるため、ハイサイドスイッチの入力はプルダウンで切れている必要がある。
pub struct Heaters {
    tim1: SimplePwmChannels<'static, TIM1>,
    tim4: SimplePwmChannels<'static, TIM4>,
    tim3: SimplePwmChannels<'static, TIM3>,
}

pub struct Pins {
    pub tim1: Peri<'static, TIM1>,
    pub pe9: Peri<'static, PE9>,
    pub pe11: Peri<'static, PE11>,
    pub pe13: Peri<'static, PE13>,
    pub pe14: Peri<'static, PE14>,
    pub tim4: Peri<'static, TIM4>,
    pub pd12: Peri<'static, PD12>,
    pub pd13: Peri<'static, PD13>,
    pub pd14: Peri<'static, PD14>,
    pub pd15: Peri<'static, PD15>,
    pub tim3: Peri<'static, TIM3>,
    pub pc6: Peri<'static, PC6>,
}

impl Heaters {
    pub fn new(p: Pins) -> Self {
        let push_pull = OutputType::PushPull;
        let mode = CountingMode::EdgeAlignedUp;
        let tim1 = SimplePwm::new(
            p.tim1,
            Some(PwmPin::new(p.pe9, push_pull)),
            Some(PwmPin::new(p.pe11, push_pull)),
            Some(PwmPin::new(p.pe13, push_pull)),
            Some(PwmPin::new(p.pe14, push_pull)),
            PWM_FREQUENCY,
            mode,
        );
        let tim4 = SimplePwm::new(
            p.tim4,
            Some(PwmPin::new(p.pd12, push_pull)),
            Some(PwmPin::new(p.pd13, push_pull)),
            Some(PwmPin::new(p.pd14, push_pull)),
            Some(PwmPin::new(p.pd15, push_pull)),
            PWM_FREQUENCY,
            mode,
        );
        let tim3 = SimplePwm::new(p.tim3, Some(PwmPin::new(p.pc6, push_pull)), None, None, None, PWM_FREQUENCY, mode);
        let mut heaters = Self { tim1: tim1.split(), tim4: tim4.split(), tim3: tim3.split() };
        heaters.set_power(&[0.0; NU]);
        heaters.tim1.ch1.enable();
        heaters.tim1.ch2.enable();
        heaters.tim1.ch3.enable();
        heaters.tim1.ch4.enable();
        heaters.tim4.ch1.enable();
        heaters.tim4.ch2.enable();
        heaters.tim4.ch3.enable();
        heaters.tim4.ch4.enable();
        heaters.tim3.ch1.enable();
        heaters
    }

    /// 電力 [W] を、電源電圧をかけ続けたときの電力に対する PWM の比にする。
    pub fn set_power(&mut self, power: &[f32; NU]) {
        let [p0, p1, p2, p3, p4, p5, p6, p7, p8] = *power;
        set(&mut self.tim1.ch1, p0);
        set(&mut self.tim1.ch2, p1);
        set(&mut self.tim1.ch3, p2);
        set(&mut self.tim1.ch4, p3);
        set(&mut self.tim4.ch1, p4);
        set(&mut self.tim4.ch2, p5);
        set(&mut self.tim4.ch3, p6);
        set(&mut self.tim4.ch4, p7);
        set(&mut self.tim3.ch1, p8);
    }
}

fn set<T: GeneralInstance4Channel>(channel: &mut SimplePwmChannel<'static, T>, power: f32) {
    let fraction = (power / FULL_POWER_W).clamp(0.0, 1.0);
    channel.set_duty_cycle((fraction * channel.max_duty_cycle() as f32) as u32);
}
