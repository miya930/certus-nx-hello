//! UART0 のシリアルコンソール。端末から 1 行ずつ受け取り、コマンドとして実行する。

pub mod commands;
mod line_editor;

use crate::config::Config;
use crate::drivers::clock::Clock;
use crate::drivers::dp83867::Dp83867;
use crate::drivers::terminal::Terminal;
use crate::traffic::Traffic;
use commands::Commands;
use line_editor::LineEditor;

/// PHY は `status` と `stats clear` で、時計は `info` の稼働時間で使う。
pub struct Console {
    editor: LineEditor,
    terminal: Terminal,
    phy: Dp83867,
    clock: Clock,
}

impl Console {
    pub fn new(terminal: Terminal, phy: Dp83867, clock: Clock) -> Self {
        Console { editor: LineEditor::new(), terminal, phy, clock }
    }

    /// 起動時の知らせのように、コマンドの外から端末に書くときに使う。
    pub fn terminal(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    pub fn prompt(&mut self) {
        self.editor.prompt(&mut self.terminal);
    }

    /// 端末から届いた文字を処理し、行が確定したらコマンドとして実行する。
    /// コマンドが書き換えるのは、動作中の設定と統計だけである。
    pub fn process_input(&mut self, config: &mut Config, traffic: &mut Traffic) {
        while let Some(line) = self.editor.read_line(Commands::complete, &mut self.terminal) {
            Commands::new(config, traffic, &mut self.terminal, self.phy, self.clock).execute(line);
            self.editor.prompt(&mut self.terminal);
        }
    }
}
