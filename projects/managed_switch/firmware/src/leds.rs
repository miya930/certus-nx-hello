//! ボードの 8 個の汎用 LED。GPIO の出力につながる。
//! 最下位に DP83867 のリンクを、最上位に動作を示す点滅を出し、残りに受け取ったフレームの数の下位を出す。

use neorv32_hal::gpio::Gpio;

const LINK: u32 = 1 << 0;
const RX_SHIFT: u32 = 1;
const RX_MASK: u32 = 0x3F << RX_SHIFT;
const HEARTBEAT: u32 = 1 << 7;

pub struct Leds {
    gpio: Gpio,
    /// GPIO に書いてある値。
    lit: u32,
}

impl Leds {
    /// 覚えている値と実際の出力をそろえるため、最初に全て消す。
    pub fn new(mut gpio: Gpio) -> Self {
        gpio.write(0);
        Leds { gpio, lit: 0 }
    }

    /// mask の LED だけを value の値にする。
    fn set(&mut self, mask: u32, value: u32) {
        let lit = (self.lit & !mask) | (value & mask);
        if lit != self.lit {
            self.lit = lit;
            self.gpio.write(lit);
        }
    }

    pub fn set_link(&mut self, up: bool) {
        self.set(LINK, if up { LINK } else { 0 });
    }

    pub fn toggle_heartbeat(&mut self) {
        self.set(HEARTBEAT, !self.lit);
    }

    pub fn show_rx_count(&mut self, count: u32) {
        self.set(RX_MASK, count << RX_SHIFT);
    }
}
