//! Stack buffer, color tables, itoa, and widget formatters (< 130 lines).

pub const OUT_CAP: usize = 1024;

#[derive(Debug)]
pub struct Out {
    buf: [u8; OUT_CAP],
    len: usize,
    full: bool,
}

impl Default for Out {
    fn default() -> Self {
        Self::new()
    }
}

impl Out {
    pub const fn new() -> Self {
        Self {
            buf: [0; OUT_CAP],
            len: 0,
            full: false,
        }
    }

    pub fn push(&mut self, b: &[u8]) {
        let end = self.len.saturating_add(b.len());
        if end <= OUT_CAP {
            self.buf[self.len..end].copy_from_slice(b);
            self.len = end;
        } else {
            self.full = true;
        }
    }

    pub fn byte(&mut self, b: u8) {
        self.push(&[b]);
    }
    pub const fn mark(&self) -> usize {
        self.len
    }
    pub const fn rollback(&mut self, mark: usize) {
        self.len = mark;
        self.full = false;
    }
    pub const fn full(&self) -> bool {
        self.full
    }
    pub const fn as_bytes(&self) -> &[u8] {
        let (left, _) = self.buf.split_at(self.len);
        left
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Dialect {
    Plain,
    Ansi,
    Tmux,
    Zsh,
    Bash,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color {
    Reset,
    Green,
    Yellow,
    Red,
    Cyan,
}

static STYLE: [[&[u8]; 5]; 5] = [
    [b"", b"", b"", b"", b""],
    [
        b"\x1b[0m",
        b"\x1b[32m",
        b"\x1b[33m",
        b"\x1b[31m",
        b"\x1b[36m",
    ],
    [
        b"#[fg=default]",
        b"#[fg=green]",
        b"#[fg=yellow]",
        b"#[fg=red]",
        b"#[fg=cyan]",
    ],
    [b"%f", b"%F{green}", b"%F{yellow}", b"%F{red}", b"%F{cyan}"],
    [
        b"\x01\x1b[0m\x02",
        b"\x01\x1b[32m\x02",
        b"\x01\x1b[33m\x02",
        b"\x01\x1b[31m\x02",
        b"\x01\x1b[36m\x02",
    ],
];

pub fn style(out: &mut Out, d: Dialect, c: Color) {
    out.push(STYLE[d as usize][c as usize]);
}

pub const fn color_for(bp: u16) -> Color {
    match bp {
        0..=5000 => Color::Green,
        5001..=8000 => Color::Yellow,
        _ => Color::Red,
    }
}

pub fn u64(out: &mut Out, mut v: u64) {
    let mut tmp = [0u8; 20];
    let mut i = tmp.len();
    loop {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.push(&tmp[i..]);
}

pub fn percent(out: &mut Out, bp: u16) {
    u64(out, u64::from(bp / 100));
    out.byte(b'%');
}

pub fn human_bytes(out: &mut Out, b: u64) {
    if b >= 1 << 30 {
        u64(out, b >> 30);
        out.byte(b'.');
        u64(out, ((b & 0x3FFF_FFFF) * 10) >> 30);
        out.byte(b'G');
    } else {
        u64(out, b >> 20);
        out.byte(b'M');
    }
}

pub fn text(out: &mut Out, d: Dialect, s: &[u8]) {
    for &b in s {
        match b {
            0..=0x1F | 0x7F => out.byte(b'?'),
            b'#' if d == Dialect::Tmux => out.push(b"##"),
            b'%' if d == Dialect::Zsh => out.push(b"%%"),
            _ => out.byte(b),
        }
    }
}

pub fn cpu(out: &mut Out, d: Dialect, pct: Option<u16>, ok: bool) {
    out.push(b"CPU ");
    if let Some(bp) = pct {
        style(out, d, color_for(bp));
        percent(out, bp);
        style(out, d, Color::Reset);
    } else if ok {
        out.push(b"--");
    } else {
        out.push(b"?");
    }
}

pub fn mem(out: &mut Out, d: Dialect, info: Option<crate::mem::MemInfo>) {
    out.push(b"MEM ");
    if let Some(m) = info {
        let used = m.total.saturating_sub(m.available);
        let bp = if m.total > 0 {
            u16::try_from((u128::from(used) * 10_000) / u128::from(m.total)).unwrap_or(10_000)
        } else {
            0
        };
        style(out, d, color_for(bp));
        human_bytes(out, used);
        out.byte(b'/');
        human_bytes(out, m.total);
        style(out, d, Color::Reset);
    } else {
        out.push(b"?");
    }
}

pub fn ssd(out: &mut Out, d: Dialect, info: Option<crate::ssd::SsdInfo>) {
    out.push(b"SSD ");
    if let Some(s) = info {
        let used = s.total.saturating_sub(s.avail);
        let bp = if s.total > 0 {
            u16::try_from((u128::from(used) * 10_000) / u128::from(s.total)).unwrap_or(10_000)
        } else {
            0
        };
        style(out, d, color_for(bp));
        human_bytes(out, used);
        out.byte(b'/');
        human_bytes(out, s.total);
        style(out, d, Color::Reset);
    } else {
        out.push(b"?");
    }
}

pub fn git(out: &mut Out, d: Dialect, info: &crate::git::GitInfo) {
    use crate::git::{GitOp, Worktree};
    out.push(b"GIT ");
    let br = &info.branch[..usize::from(info.branch_len)];
    if br.is_empty() {
        out.push(b":");
        if let Some(oid) = &info.oid {
            for &b in oid {
                out.byte(b"0123456789abcdef"[usize::from(b >> 4)]);
                out.byte(b"0123456789abcdef"[usize::from(b & 0x0F)]);
            }
        }
    } else {
        text(out, d, br);
        if let Some(oid) = &info.oid {
            out.push(b":");
            for &b in &oid[..4] {
                out.byte(b"0123456789abcdef"[usize::from(b >> 4)]);
                out.byte(b"0123456789abcdef"[usize::from(b & 0x0F)]);
            }
        }
    }
    match info.op {
        GitOp::None => {}
        GitOp::Merge => out.push(b"|MERGE"),
        GitOp::Rebase => out.push(b"|REBASE"),
        GitOp::CherryPick => out.push(b"|CHERRY"),
        GitOp::Revert => out.push(b"|REVERT"),
        GitOp::Bisect => out.push(b"|BISECT"),
    }
    match info.worktree {
        Worktree::Clean => {
            style(out, d, Color::Green);
            out.push(" ✓".as_bytes());
            style(out, d, Color::Reset);
        }
        Worktree::Modified => {
            style(out, d, Color::Yellow);
            out.push(" ✗".as_bytes());
            style(out, d, Color::Reset);
        }
        Worktree::Unknown => out.push(b" ?"),
    }
}

pub fn git_err(out: &mut Out, _d: Dialect) {
    out.push(b"GIT ?");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_buffer_rollback() {
        let mut out = Out::new();
        out.push(b"hello");
        let m = out.mark();
        out.push(b" world");
        out.rollback(m);
        assert_eq!(out.as_bytes(), b"hello");
    }

    #[test]
    fn human_bytes_formatting() {
        let mut out = Out::new();
        human_bytes(&mut out, 8 * (1 << 30));
        assert_eq!(out.as_bytes(), b"8.0G");
        out.rollback(0);
        human_bytes(&mut out, 512 * (1 << 20));
        assert_eq!(out.as_bytes(), b"512M");
    }

    #[test]
    fn text_sanitization() {
        let mut out = Out::new();
        text(&mut out, Dialect::Ansi, b"\x1b[31mbad\x07");
        assert_eq!(out.as_bytes(), b"?[31mbad?");
        out.rollback(0);
        text(&mut out, Dialect::Tmux, b"branch#1");
        assert_eq!(out.as_bytes(), b"branch##1");
    }
}
