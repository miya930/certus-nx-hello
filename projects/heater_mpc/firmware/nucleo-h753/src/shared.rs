use core::cell::RefCell;

use control::model::{AMBIENT_C, NU};
use control::protocol::Mode;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

/// 優先度の違う executor のタスクの間で共有する状態。
/// 割り込みを禁止して読み書きするので、どちらの優先度から触っても値が崩れない。
#[derive(Clone, Copy)]
pub struct State {
    pub mode: Mode,
    pub fault: bool,
    /// 平滑化したサーミスタの温度 [°C]
    pub temperature: [f32; NU],
    pub setpoint: [f32; NU],
    /// 制御器が最後に出した電力 [W]
    pub power: [f32; NU],
    pub compute_ms: f32,
}

impl State {
    /// ヒーターに実際にかける電力。止めているときと異常のときは制御器の出力に関わらず 0 にする。
    pub fn applied_power(&self) -> [f32; NU] {
        if self.mode == Mode::Off || self.fault { [0.0; NU] } else { self.power }
    }
}

static STATE: Mutex<CriticalSectionRawMutex, RefCell<State>> = Mutex::new(RefCell::new(State {
    mode: Mode::Off,
    fault: false,
    temperature: [AMBIENT_C; NU],
    setpoint: [AMBIENT_C; NU],
    power: [0.0; NU],
    compute_ms: 0.0,
}));

pub fn read() -> State {
    STATE.lock(|s| *s.borrow())
}

pub fn update<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.lock(|s| f(&mut s.borrow_mut()))
}
