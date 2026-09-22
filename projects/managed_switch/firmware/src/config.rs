//! 動作中の設定と、SPI Flash に保存した設定。
//! コンソールのコマンドは動作中の設定を書き換え、メインループが apply でスイッチに反映する。

use crate::host::Host;
use crate::memory_map::SWITCH;
use managed_switch_logic::settings::{self, Record, Settings, RECORD_BYTES};
use mt25q::Mt25q;
use neorv32_hal::spi::Spi;

/// FPGA のビットストリームは Flash の先頭から置かれる。
/// 設定は、ビットストリームと重ならない最後の区画に置く。
const OFFSET: u32 = mt25q::CAPACITY_BYTES - mt25q::SUBSECTOR_BYTES;
const _: () = assert!(RECORD_BYTES <= mt25q::PAGE_BYTES);

pub struct Config {
    flash: Mt25q<Spi>,
    current: Settings,
    /// 保存したことがなければ None になる。
    saved: Option<Settings>,
    /// 最後にスイッチに反映した設定。まだ反映していなければ None になる。
    applied: Option<Settings>,
}

impl Config {
    /// Flash に保存した設定を読む。保存していなければ既定値で動く。
    pub fn load(mut flash: Mt25q<Spi>) -> Self {
        let mut record: Record = [0; RECORD_BYTES];
        let Ok(()) = flash.read(OFFSET, &mut record);
        let saved = Settings::decode(&record);
        Config { flash, current: saved.unwrap_or(settings::DEFAULT), saved, applied: None }
    }

    pub fn current(&self) -> &Settings {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut Settings {
        &mut self.current
    }

    pub fn saved(&self) -> Option<&Settings> {
        self.saved.as_ref()
    }

    pub fn restore_defaults(&mut self) {
        self.current = settings::DEFAULT;
    }

    pub fn save(&mut self) {
        let Ok(()) = self.flash.erase_subsector(OFFSET);
        let Ok(()) = self.flash.program(OFFSET, &self.current.encode());
        self.saved = Some(self.current);
    }

    /// 動作中の設定が、最後に反映したものと違えば反映する。
    /// ミラーリングは、指定した 1 つのポートだけをプロミスキャスにして実現する。
    pub fn apply(&mut self, host: &mut Host) {
        if self.applied == Some(self.current) {
            return;
        }
        host.configure(&self.current);
        SWITCH.set_promiscuous(self.current.mirror.map_or(0, |port| 1 << port));
        self.applied = Some(self.current);
    }
}
