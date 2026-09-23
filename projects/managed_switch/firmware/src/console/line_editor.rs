//! 端末からの入力を 1 行ずつ受け取る。行の編集、ヒストリー、タブ補完を、端末のエスケープシーケンスで行う。
//! 編集中の行は、端末に描き直す。

use crate::drivers::terminal::{Style, Terminal};

pub const PROMPT: &str = "switch> ";
pub const LINE_BYTES: usize = 64;
const HISTORY_LINES: usize = 8;
/// タブ補完で一度に扱う候補の数。コマンドの数より多くしておく。
const MAX_CANDIDATES: usize = 16;

const CTRL_A: u8 = 0x01;
const CTRL_B: u8 = 0x02;
const CTRL_C: u8 = 0x03;
const CTRL_D: u8 = 0x04;
const CTRL_E: u8 = 0x05;
const CTRL_F: u8 = 0x06;
const BELL: u8 = 0x07;
const BACKSPACE: u8 = 0x08;
const TAB: u8 = 0x09;
const LF: u8 = 0x0A;
const CTRL_K: u8 = 0x0B;
const CTRL_L: u8 = 0x0C;
const CR: u8 = 0x0D;
const CTRL_N: u8 = 0x0E;
const CTRL_P: u8 = 0x10;
const CTRL_U: u8 = 0x15;
const CTRL_W: u8 = 0x17;
const ESC: u8 = 0x1B;
const DEL: u8 = 0x7F;

/// 入力中の行の、単語より前の部分を受け取り、その位置に入りうる単語を全て渡す。
pub type Completer = fn(context: &str, emit: &mut dyn FnMut(&'static str));

#[derive(Clone, Copy)]
struct Line {
    bytes: [u8; LINE_BYTES],
    len: usize,
}

impl Line {
    const EMPTY: Line = Line {
        bytes: [0; LINE_BYTES],
        len: 0,
    };

    /// 入力は印字できる ASCII に限っているため、UTF-8 として常に正しい。
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

/// エスケープシーケンスは、ESC [ に数字と終わりの文字が続く形と、ESC O に 1 文字が続く形を読む。
#[derive(Clone, Copy)]
enum Escape {
    None,
    Start,
    Csi(u8),
    Ss3,
}

enum Key {
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Delete,
}

pub struct LineEditor {
    line: Line,
    cursor: usize,
    history: [Line; HISTORY_LINES],
    history_count: usize,
    /// ヒストリーをさかのぼっている間の位置。0 が一番新しい行を指す。
    browse: Option<usize>,
    /// ヒストリーをさかのぼる前に入力していた行。
    draft: Line,
    /// 確定した行。次の行を読むまで、呼び出し側に貸す。
    entered: Line,
    escape: Escape,
    /// CR の直後の LF は、同じ改行の続きとして読み捨てる。
    after_cr: bool,
}

impl LineEditor {
    pub const fn new() -> Self {
        LineEditor {
            line: Line::EMPTY,
            cursor: 0,
            history: [Line::EMPTY; HISTORY_LINES],
            history_count: 0,
            browse: None,
            draft: Line::EMPTY,
            entered: Line::EMPTY,
            escape: Escape::None,
            after_cr: false,
        }
    }

    pub fn prompt(&self, out: &mut Terminal) {
        out.puts_styled(Style::PROMPT, PROMPT);
    }

    /// 届いている文字を処理する。行が確定したらその行を返し、確定する前に文字が尽きたら None を返す。
    pub fn read_line(&mut self, complete: Completer, out: &mut Terminal) -> Option<&str> {
        while let Some(byte) = out.read_byte() {
            if self.feed(byte, complete, out) {
                return Some(self.entered.as_str());
            }
        }
        None
    }

    /// 1 バイトを処理し、行が確定したら真を返す。
    fn feed(&mut self, byte: u8, complete: Completer, out: &mut Terminal) -> bool {
        let after_cr = self.after_cr;
        self.after_cr = byte == CR;

        match self.escape {
            Escape::None => {}
            Escape::Start => {
                self.escape = match byte {
                    b'[' => Escape::Csi(0),
                    b'O' => Escape::Ss3,
                    _ => Escape::None,
                };
                return false;
            }
            Escape::Csi(param) => {
                if byte.is_ascii_digit() {
                    self.escape = Escape::Csi(param.saturating_mul(10).saturating_add(byte - b'0'));
                    return false;
                }
                self.escape = Escape::None;
                let key = match (byte, param) {
                    (b'A', _) => Some(Key::Up),
                    (b'B', _) => Some(Key::Down),
                    (b'C', _) => Some(Key::Right),
                    (b'D', _) => Some(Key::Left),
                    (b'H', _) | (b'~', 1) | (b'~', 7) => Some(Key::Home),
                    (b'F', _) | (b'~', 4) | (b'~', 8) => Some(Key::End),
                    (b'~', 3) => Some(Key::Delete),
                    _ => None,
                };
                if let Some(key) = key {
                    self.key(key, out);
                }
                return false;
            }
            Escape::Ss3 => {
                self.escape = Escape::None;
                let key = match byte {
                    b'A' => Some(Key::Up),
                    b'B' => Some(Key::Down),
                    b'C' => Some(Key::Right),
                    b'D' => Some(Key::Left),
                    b'H' => Some(Key::Home),
                    b'F' => Some(Key::End),
                    _ => None,
                };
                if let Some(key) = key {
                    self.key(key, out);
                }
                return false;
            }
        }

        match byte {
            ESC => self.escape = Escape::Start,
            CR => {
                self.enter(out);
                return true;
            }
            LF if after_cr => {}
            LF => {
                self.enter(out);
                return true;
            }
            BACKSPACE | DEL => self.backspace(out),
            TAB => self.complete(complete, out),
            CTRL_A => self.key(Key::Home, out),
            CTRL_E => self.key(Key::End, out),
            CTRL_B => self.key(Key::Left, out),
            CTRL_F => self.key(Key::Right, out),
            CTRL_P => self.key(Key::Up, out),
            CTRL_N => self.key(Key::Down, out),
            CTRL_D => self.key(Key::Delete, out),
            CTRL_C => {
                out.puts("^C\n");
                self.line = Line::EMPTY;
                self.cursor = 0;
                self.browse = None;
                self.prompt(out);
            }
            CTRL_K => {
                self.line.len = self.cursor;
                self.refresh(out);
            }
            CTRL_U => self.remove(0, self.cursor, out),
            CTRL_W => {
                let start = self.word_start(true);
                self.remove(start, self.cursor, out);
            }
            CTRL_L => {
                // 画面を消し、カーソルを左上に戻してから行を描き直す。
                out.puts("\x1b[2J\x1b[H");
                self.refresh(out);
            }
            b' '..=b'~' => self.insert(&[byte], out),
            _ => {}
        }
        false
    }

    fn key(&mut self, key: Key, out: &mut Terminal) {
        match key {
            Key::Left if self.cursor > 0 => self.move_to(self.cursor - 1, out),
            Key::Right if self.cursor < self.line.len => self.move_to(self.cursor + 1, out),
            Key::Home => self.move_to(0, out),
            Key::End => self.move_to(self.line.len, out),
            Key::Delete if self.cursor < self.line.len => {
                self.remove(self.cursor, self.cursor + 1, out)
            }
            Key::Up => self.browse_history(true, out),
            Key::Down => self.browse_history(false, out),
            _ => {}
        }
    }

    /// 行頭から描き直し、行末の残りを消してから、カーソルを編集位置へ戻す。
    fn refresh(&self, out: &mut Terminal) {
        out.put(CR);
        self.prompt(out);
        out.puts(self.line.as_str());
        out.puts("\x1b[K");
        Self::cursor_back(self.line.len - self.cursor, out);
    }

    fn cursor_back(columns: usize, out: &mut Terminal) {
        if columns > 0 {
            out.puts("\x1b[");
            out.put_dec(columns as u32);
            out.put(b'D');
        }
    }

    fn move_to(&mut self, position: usize, out: &mut Terminal) {
        self.cursor = position;
        self.refresh(out);
    }

    fn set_line(&mut self, line: Line, out: &mut Terminal) {
        self.line = line;
        self.cursor = line.len;
        self.refresh(out);
    }

    fn insert(&mut self, text: &[u8], out: &mut Terminal) {
        let count = text.len().min(LINE_BYTES - self.line.len);
        if count == 0 {
            out.put(BELL);
            return;
        }
        let (cursor, len) = (self.cursor, self.line.len);
        self.line.bytes.copy_within(cursor..len, cursor + count);
        self.line.bytes[cursor..cursor + count].copy_from_slice(&text[..count]);
        self.line.len += count;
        self.cursor += count;
        if self.cursor == self.line.len {
            // 行末への追加は、描き直さずにそのまま表示する。
            for &byte in &text[..count] {
                out.put(byte);
            }
        } else {
            self.refresh(out);
        }
    }

    fn remove(&mut self, start: usize, end: usize, out: &mut Terminal) {
        if start == end {
            return;
        }
        self.line.bytes.copy_within(end..self.line.len, start);
        self.line.len -= end - start;
        self.cursor = start;
        self.refresh(out);
    }

    fn backspace(&mut self, out: &mut Terminal) {
        if self.cursor > 0 {
            self.remove(self.cursor - 1, self.cursor, out);
        }
    }

    /// カーソルより前にある単語の先頭を返す。skip_spaces が真なら、直前の空白も単語に含める。
    fn word_start(&self, skip_spaces: bool) -> usize {
        let bytes = &self.line.bytes[..self.cursor];
        let mut start = self.cursor;
        if skip_spaces {
            while start > 0 && bytes[start - 1] == b' ' {
                start -= 1;
            }
        }
        while start > 0 && bytes[start - 1] != b' ' {
            start -= 1;
        }
        start
    }

    fn enter(&mut self, out: &mut Terminal) {
        out.puts("\n");
        let line = self.line;
        if !line.as_str().trim().is_empty() {
            let newest = &self.history[0];
            if self.history_count == 0 || newest.as_str() != line.as_str() {
                self.history.copy_within(0..HISTORY_LINES - 1, 1);
                self.history[0] = line;
                self.history_count = (self.history_count + 1).min(HISTORY_LINES);
            }
        }
        self.browse = None;
        self.line = Line::EMPTY;
        self.cursor = 0;
        self.entered = line;
    }

    fn browse_history(&mut self, older: bool, out: &mut Terminal) {
        let next = match (self.browse, older) {
            (None, true) if self.history_count > 0 => Some(0),
            (Some(index), true) if index + 1 < self.history_count => Some(index + 1),
            (Some(0), false) => None,
            (Some(index), false) => Some(index - 1),
            _ => {
                out.put(BELL);
                return;
            }
        };
        if self.browse.is_none() {
            self.draft = self.line;
        }
        self.browse = next;
        let line = match next {
            Some(index) => self.history[index],
            None => self.draft,
        };
        self.set_line(line, out);
    }

    /// 候補が 1 つなら単語を完成させ、複数なら共通の部分まで延ばし、それ以上延びなければ一覧を出す。
    fn complete(&mut self, complete: Completer, out: &mut Terminal) {
        let start = self.word_start(false);
        let mut candidates: [&'static str; MAX_CANDIDATES] = [""; MAX_CANDIDATES];
        let mut count = 0;
        {
            let line = self.line.as_str();
            let (context, prefix) = (&line[..start], &line[start..self.cursor]);
            complete(context, &mut |word| {
                if word.starts_with(prefix) && count < MAX_CANDIDATES {
                    candidates[count] = word;
                    count += 1;
                }
            });
        }
        let typed = self.cursor - start;
        match count {
            0 => out.put(BELL),
            1 => {
                let rest = &candidates[0].as_bytes()[typed..];
                self.insert(rest, out);
                self.insert(b" ", out);
            }
            _ => {
                let common =
                    candidates[1..count]
                        .iter()
                        .fold(candidates[0].len(), |common, word| {
                            common.min(
                                candidates[0]
                                    .bytes()
                                    .zip(word.bytes())
                                    .take_while(|(a, b)| a == b)
                                    .count(),
                            )
                        });
                if common > typed {
                    self.insert(&candidates[0].as_bytes()[typed..common], out);
                    return;
                }
                out.puts("\n");
                for word in &candidates[..count] {
                    out.puts(word);
                    out.puts("  ");
                }
                out.puts("\n");
                self.refresh(out);
            }
        }
    }
}
