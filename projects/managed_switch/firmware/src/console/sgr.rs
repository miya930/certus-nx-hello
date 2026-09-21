//! 端末の文字の色と太さを、ANSI のエスケープシーケンス (SGR) で指定する。
//! 色の指定は表示の幅を持たないため、列をそろえるときは文字数に数えない。

use super::term::puts;

pub const BOLD: &str = "\x1b[1m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN: &str = "\x1b[36m";
pub const RESET: &str = "\x1b[0m";

pub fn puts_styled(style: &str, text: &str) {
    puts(style);
    puts(text);
    puts(RESET);
}

/// 表の列をそろえるため、色を付けた文字列の後ろを空白で埋める。
pub fn puts_styled_padded(style: &str, text: &str, width: usize) {
    puts_styled(style, text);
    for _ in text.len()..width {
        puts(" ");
    }
}
