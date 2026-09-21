//! UART0 のシリアルコンソール。端末から 1 行ずつ受け取り、コマンドとして実行する。

pub mod commands;
mod input;
mod output;
mod style;

pub use output::Output;
pub use style::Style;

use commands::{Commands, State};
use input::Input;
use neorv32_hal::uart::Uart;

/// UART の受信は入力が、送信は出力が持つ。
pub struct Console {
    input: Input,
    output: Output,
}

impl Console {
    pub fn new(uart: Uart) -> Self {
        let (tx, rx) = uart.split();
        Console {
            input: Input::new(rx),
            output: Output::new(tx),
        }
    }

    /// 起動時の知らせのように、コマンドの外から端末に書くときに使う。
    pub fn output(&mut self) -> &mut Output {
        &mut self.output
    }

    pub fn prompt(&mut self) {
        self.input.prompt(&mut self.output);
    }

    /// 端末から届いた文字を全て処理する。設定を変えるコマンドを実行したときは真を返す。
    pub fn poll(&mut self, state: &mut State) -> bool {
        let mut changed = false;
        while let Some(line) = self.input.read_line(Commands::complete, &mut self.output) {
            changed |= Commands::new(state, &mut self.output).execute(line);
            self.input.prompt(&mut self.output);
        }
        changed
    }
}
