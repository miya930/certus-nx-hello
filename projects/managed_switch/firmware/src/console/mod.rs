//! UART0 のシリアルコンソール。端末から 1 行ずつ受け取り、コマンドとして実行する。

pub mod commands;
mod input;
mod output;
mod style;

pub use output::Output;
pub use style::Style;

use crate::config::Config;
use crate::dp83867::Dp83867;
use crate::traffic::Traffic;
use commands::Commands;
use input::Input;
use neorv32_hal::{mtime::Mtime, uart::Uart};

/// UART の受信は入力が、送信は出力が持つ。
/// PHY と時計は、`status` のリンクと `info` の稼働時間を読むためだけに持つ。
pub struct Console {
    input: Input,
    output: Output,
    phy: Dp83867,
    mtime: Mtime,
}

impl Console {
    pub fn new(uart: Uart, phy: Dp83867, mtime: Mtime) -> Self {
        let (tx, rx) = uart.split();
        Console { input: Input::new(rx), output: Output::new(tx), phy, mtime }
    }

    /// 起動時の知らせのように、コマンドの外から端末に書くときに使う。
    pub fn output(&mut self) -> &mut Output {
        &mut self.output
    }

    pub fn prompt(&mut self) {
        self.input.prompt(&mut self.output);
    }

    /// 端末から届いた文字を処理し、行が確定したらコマンドとして実行する。
    /// コマンドが書き換えるのは、動作中の設定と統計だけである。
    pub fn process_input(&mut self, config: &mut Config, traffic: &mut Traffic) {
        while let Some(line) = self.input.read_line(Commands::complete, &mut self.output) {
            Commands::new(config, traffic, &mut self.output, self.phy, self.mtime).execute(line);
            self.input.prompt(&mut self.output);
        }
    }
}
