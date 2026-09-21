//! UART0 のシリアルコンソール。端末から 1 行ずつ受け取り、コマンドとして実行する。

pub mod commands;
mod input;
pub mod output;
pub mod style;

use commands::State;
use core::cell::RefCell;
use critical_section::Mutex;
use input::LineEditor;
use neorv32_hal::uart::Uart;

/// 入力と出力のどちらも使うため、UART は起動時に受け取って共有する。
static UART: Mutex<RefCell<Option<Uart>>> = Mutex::new(RefCell::new(None));

pub fn init(uart: Uart) {
    critical_section::with(|cs| UART.borrow_ref_mut(cs).replace(uart));
}

fn with_uart<R>(f: impl FnOnce(&mut Uart) -> R) -> R {
    critical_section::with(|cs| f(UART.borrow_ref_mut(cs).as_mut().expect("console::init was not called")))
}

pub struct Console {
    editor: LineEditor,
}

impl Console {
    /// プロンプトを出して、入力を待ち始める。
    pub fn new() -> Self {
        let editor = LineEditor::new();
        editor.prompt();
        Console { editor }
    }

    /// 端末から届いた文字を全て処理する。設定を変えるコマンドを実行したときは真を返す。
    pub fn poll(&mut self, state: &mut State) -> bool {
        let mut changed = false;
        while let Some(byte) = input::get() {
            if let Some(line) = self.editor.feed(byte, commands::complete) {
                changed |= commands::execute(line, state);
                self.editor.prompt();
            }
        }
        changed
    }
}
