//! 端末に出す文字の見た目。役割ごとに色と太さを決め、ANSI のエスケープシーケンスの SGR で指定する。
//! 色の指定は表示の幅を持たないため、列をそろえるときは文字数に数えない。

#[derive(Clone, Copy)]
pub struct Style(&'static str);

impl Style {
    /// 表の見出しと、コマンドの名前。
    pub const HEADING: Style = Style("\x1b[1m");
    pub const PROMPT: Style = Style("\x1b[36m");
    /// 操作が済んだことと、リンクが上がっていること。
    pub const OK: Style = Style("\x1b[32m");
    /// 見てほしいこと。保存していない設定や、0 でない破棄とエラーの数に使う。
    pub const WARNING: Style = Style("\x1b[33m");
    /// 受け付けなかった入力と、リンクが下がっていること。
    pub const ERROR: Style = Style("\x1b[31m");
    /// 色も太さも付けない、既定の見た目。
    pub const NORMAL: Style = Style("\x1b[0m");

    pub fn code(self) -> &'static str {
        self.0
    }
}
