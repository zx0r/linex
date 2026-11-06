//! Stack buffer, color tables, itoa, and widget formatters.

pub const OUT_CAP: usize = 1024;

#[rustfmt::skip]
#[derive(Debug)]
pub struct Out { buf: [u8; OUT_CAP], len: usize, full: bool }

impl Default for Out {
    fn default() -> Self {
        Self::new()
    }
}

#[rustfmt::skip]
impl Out {
    pub const fn new() -> Self { Self { buf: [0; OUT_CAP], len: 0, full: false } }
    pub fn push(&mut self, b: &[u8]) {
        let end = self.len.saturating_add(b.len());
        if end <= OUT_CAP { self.buf[self.len..end].copy_from_slice(b); self.len = end; } else { self.full = true; }
    }
    pub fn byte(&mut self, b: u8) { self.push(&[b]); }
    pub const fn mark(&self) -> usize { self.len }
    pub const fn rollback(&mut self, mark: usize) { self.len = mark; self.full = false; }
    pub const fn full(&self) -> bool { self.full }
    pub const fn as_bytes(&self) -> &[u8] { self.buf.split_at(self.len).0 }
}

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Dialect { Plain, Ansi, Tmux }

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color { Reset, Green, Yellow, Red, Cyan, Blue, Magenta, Pink }

#[rustfmt::skip]
pub static STYLE_MATRIX: [[&[u8]; 8]; 3] = [
    [b"", b"", b"", b"", b"", b"", b"", b""],
    [b"\x1b[0m", b"\x1b[38;5;84m", b"\x1b[38;5;227m", b"\x1b[38;5;203m", b"\x1b[38;5;87m", b"\x1b[38;5;75m", b"\x1b[38;5;135m", b"\x1b[38;5;205m"],
    [b"#[default]", b"#[fg=colour84]", b"#[fg=colour227]", b"#[fg=colour203]", b"#[fg=colour87]", b"#[fg=colour75]", b"#[fg=colour135]", b"#[fg=colour205]"],
];

pub const ICON_CPU: &[u8] = "\u{f035b}".as_bytes();
pub const ICON_MEM: &[u8] = "\u{efc5}".as_bytes();
pub const ICON_SSD: &[u8] = "\u{f02ca}".as_bytes();

pub fn style(out: &mut Out, d: Dialect, c: Color) {
    out.push(STYLE_MATRIX[d as usize][c as usize]);
}

#[rustfmt::skip]
pub const fn color_for_cpu(bp: u16) -> Color { match bp { 0..=5000 => Color::Green, 5001..=8000 => Color::Yellow, _ => Color::Red } }
#[rustfmt::skip]
pub const fn color_for_mem(bp: u16) -> Color { match bp { 0..=7000 => Color::Green, 7001..=8500 => Color::Yellow, _ => Color::Red } }
#[rustfmt::skip]
pub const fn color_for_ssd(bp: u16) -> Color { match bp { 0..=7500 => Color::Green, 7501..=9000 => Color::Yellow, _ => Color::Red } }

#[rustfmt::skip]
pub fn u64(out: &mut Out, mut v: u64) {
    let mut tmp = [0u8; 20]; let mut i = tmp.len();
    loop { i -= 1; tmp[i] = b'0' + (v % 10) as u8; v /= 10; if v == 0 { break; } }
    out.push(&tmp[i..]);
}

pub fn percent(out: &mut Out, bp: u16) {
    u64(out, u64::from(bp / 100));
    out.byte(b'%');
}

#[rustfmt::skip]
pub fn human_bytes(out: &mut Out, b: u64) {
    if b >= 1 << 30 {
        u64(out, b >> 30); out.byte(b'.'); u64(out, ((b & 0x3FFF_FFFF) * 10) >> 30); out.byte(b'G');
    } else {
        u64(out, b >> 20); out.byte(b'M');
    }
}

#[rustfmt::skip]
pub fn text(out: &mut Out, d: Dialect, s: &[u8]) {
    for &b in s {
        match b {
            0..=0x1F | 0x7F => out.byte(b'?'),
            b'#' if d == Dialect::Tmux => out.push(b"##"),
            _ => out.byte(b),
        }
    }
}

#[rustfmt::skip]
pub fn cpu(out: &mut Out, d: Dialect, icons: bool, pct: Option<u16>, ok: bool) {
    if let Some(bp) = pct {
        style(out, d, color_for_cpu(bp));
        if icons { out.push(ICON_CPU); out.byte(b' '); } else { out.push(b"CPU "); }
        percent(out, bp);
        style(out, d, Color::Reset);
    } else {
        if icons { out.push(ICON_CPU); out.byte(b' '); } else { out.push(b"CPU "); }
        out.push(if ok { b"--" } else { b"?" });
    }
}

#[rustfmt::skip]
pub fn mem(out: &mut Out, d: Dialect, icons: bool, bytes: bool, used: u64, total: u64) {
    let bp = if total > 0 { u16::try_from((u128::from(used) * 10_000) / u128::from(total)).unwrap_or(10_000) } else { 0 };
    style(out, d, color_for_mem(bp));
    if icons { out.push(ICON_MEM); out.byte(b' '); } else { out.push(b"MEM "); }
    if bytes { human_bytes(out, used); out.byte(b'/'); human_bytes(out, total); } else { percent(out, bp); }
    style(out, d, Color::Reset);
}

#[rustfmt::skip]
pub fn mem_err(out: &mut Out, icons: bool) {
    if icons { out.push(ICON_MEM); out.byte(b' '); } else { out.push(b"MEM "); }
    out.push(b"?");
}

#[rustfmt::skip]
pub fn ssd(out: &mut Out, d: Dialect, icons: bool, bytes: bool, used: u64, total: u64) {
    let bp = if total > 0 { u16::try_from((u128::from(used) * 10_000) / u128::from(total)).unwrap_or(10_000) } else { 0 };
    style(out, d, color_for_ssd(bp));
    if icons { out.push(ICON_SSD); out.byte(b' '); } else { out.push(b"SSD "); }
    if bytes { human_bytes(out, used); out.byte(b'/'); human_bytes(out, total); } else { percent(out, bp); }
    style(out, d, Color::Reset);
}

#[rustfmt::skip]
pub fn ssd_err(out: &mut Out, icons: bool) {
    if icons { out.push(ICON_SSD); out.byte(b' '); } else { out.push(b"SSD "); }
    out.push(b"?");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn fmt_tests() {
        let mut out = Out::new();
        out.push(b"hello"); let m = out.mark(); out.push(b" world"); out.rollback(m); assert_eq!(out.as_bytes(), b"hello");
        out.rollback(0); human_bytes(&mut out, 8 * (1 << 30)); assert_eq!(out.as_bytes(), b"8.0G");
        out.rollback(0); human_bytes(&mut out, 512 * (1 << 20)); assert_eq!(out.as_bytes(), b"512M");
        out.rollback(0); text(&mut out, Dialect::Ansi, b"\x1b[31mbad\x07"); assert_eq!(out.as_bytes(), b"?[31mbad?");
        out.rollback(0); text(&mut out, Dialect::Tmux, b"branch#1"); assert_eq!(out.as_bytes(), b"branch##1");
    }
}
