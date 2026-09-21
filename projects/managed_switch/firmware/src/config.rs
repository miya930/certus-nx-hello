//! 動作中の設定と、SPI Flash に保存した設定。
//! コンソールのコマンドは動作中の設定を書き換え、主ループが apply でスイッチに反映する。

use crate::host::Host;
use crate::memory_map::SWITCH_CORE;
use crate::mt25q::Mt25q;
use crate::settings::{self, Settings};
use neorv32_hal::spi::Spi;

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
        let saved = Settings::load(&mut flash);
        Config {
            flash,
            current: saved.unwrap_or(settings::DEFAULT),
            saved,
            applied: None,
        }
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
        let Ok(()) = self.current.save(&mut self.flash);
        self.saved = Some(self.current);
    }

    /// 動作中の設定が、最後に反映したものと違えば反映する。
    /// ミラーリングは、指定した 1 つのポートだけをプロミスキャスにして実現する。
    pub fn apply(&mut self, host: &mut Host) {
        if self.applied == Some(self.current) {
            return;
        }
        host.configure(&self.current);
        SWITCH_CORE.set_promiscuous(self.current.mirror.map_or(0, |port| 1 << port));
        self.applied = Some(self.current);
    }
}
