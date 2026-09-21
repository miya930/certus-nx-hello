//! UART0 のシリアルコンソール。端末から 1 行ずつ受け取り、コマンドとして実行する。

pub mod commands;
mod line_editor;
pub mod sgr;
pub mod term;

use commands::State;
use line_editor::LineEditor;

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
        while let Some(byte) = term::get() {
            if let Some(line) = self.editor.feed(byte, commands::complete) {
                changed |= commands::execute(line, state);
                self.editor.prompt();
            }
        }
        changed
    }
}
