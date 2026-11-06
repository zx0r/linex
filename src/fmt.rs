//! Stack buffer, color tables, itoa, and widget formatters.

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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color {
    Reset,
    Green,
    Yellow,
    Red,
    Cyan,
    Blue,
    Magenta,
}

pub static STYLE_MATRIX: [[&[u8]; 7]; 3] = [
    // Plain
    [b"", b"", b"", b"", b"", b"", b""],
    // Ansi
    [
        b"\x1b[0m",
        b"\x1b[32m",
        b"\x1b[33m",
        b"\x1b[31m",
        b"\x1b[36m",
        b"\x1b[34m",
        b"\x1b[35m",
    ],
    // Tmux
    [
        b"#[default]",
        b"#[fg=green]",
        b"#[fg=yellow]",
        b"#[fg=red]",
        b"#[fg=cyan]",
        b"#[fg=blue]",
        b"#[fg=magenta]",
    ],
];

pub static STYLE: [[&[u8]; 7]; 3] = STYLE_MATRIX;

pub const ICON_CPU: &[u8] = "\u{f035b}".as_bytes(); // 󰍛 [0xF3, 0xB0, 0x8D, 0x9B]
pub const ICON_MEM: &[u8] = "\u{efc5}".as_bytes(); //  [0xEE, 0xBF, 0x85]
pub const ICON_SSD: &[u8] = "\u{f02ca}".as_bytes(); // 󰋊 [0xF3, 0xB0, 0x8B, 0x8A]
pub const ICON_GIT: &[u8] = "\u{f418}".as_bytes(); //  [0xEF, 0x90, 0x98]

pub fn style(out: &mut Out, d: Dialect, c: Color) {
    out.push(STYLE_MATRIX[d as usize][c as usize]);
}

pub const fn color_for_cpu(bp: u16) -> Color {
    match bp {
        0..=5000 => Color::Green,
        5001..=8000 => Color::Yellow,
        _ => Color::Red,
    }
}

pub const fn color_for_mem(bp: u16) -> Color {
    match bp {
        0..=7000 => Color::Green,
        7001..=8500 => Color::Yellow,
        _ => Color::Red,
    }
}

pub const fn color_for_ssd(bp: u16) -> Color {
    match bp {
        0..=7500 => Color::Green,
        7501..=9000 => Color::Yellow,
        _ => Color::Red,
    }
}

pub const fn color_for(bp: u16) -> Color {
    color_for_cpu(bp)
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
            _ => out.byte(b),
        }
    }
}

pub fn cpu(out: &mut Out, d: Dialect, icons: bool, pct: Option<u16>, ok: bool) {
    if let Some(bp) = pct {
        style(out, d, color_for_cpu(bp));
        if icons {
            out.push(ICON_CPU);
            out.byte(b' ');
        } else {
            out.push(b"CPU ");
        }
        percent(out, bp);
        style(out, d, Color::Reset);
    } else {
        if icons {
            out.push(ICON_CPU);
            out.byte(b' ');
        } else {
            out.push(b"CPU ");
        }
        if ok {
            out.push(b"--");
        } else {
            out.push(b"?");
        }
    }
}

pub fn mem(out: &mut Out, d: Dialect, icons: bool, bytes: bool, info: Option<crate::mem::MemInfo>) {
    if let Some(m) = info {
        let used = m.total.saturating_sub(m.available);
        let bp = if m.total > 0 {
            u16::try_from((u128::from(used) * 10_000) / u128::from(m.total)).unwrap_or(10_000)
        } else {
            0
        };
        style(out, d, color_for_mem(bp));
        if icons {
            out.push(ICON_MEM);
            out.byte(b' ');
        } else {
            out.push(b"MEM ");
        }
        if bytes {
            human_bytes(out, used);
            out.byte(b'/');
            human_bytes(out, m.total);
        } else {
            percent(out, bp);
        }
        style(out, d, Color::Reset);
    } else {
        if icons {
            out.push(ICON_MEM);
            out.byte(b' ');
        } else {
            out.push(b"MEM ");
        }
        out.push(b"?");
    }
}

pub fn ssd(out: &mut Out, d: Dialect, icons: bool, bytes: bool, info: Option<crate::ssd::SsdInfo>) {
    if let Some(s) = info {
        let used = s.total.saturating_sub(s.avail);
        let bp = if s.total > 0 {
            u16::try_from((u128::from(used) * 10_000) / u128::from(s.total)).unwrap_or(10_000)
        } else {
            0
        };
        style(out, d, color_for_ssd(bp));
        if icons {
            out.push(ICON_SSD);
            out.byte(b' ');
        } else {
            out.push(b"SSD ");
        }
        if bytes {
            human_bytes(out, used);
            out.byte(b'/');
            human_bytes(out, s.total);
        } else {
            percent(out, bp);
        }
        style(out, d, Color::Reset);
    } else {
        if icons {
            out.push(ICON_SSD);
            out.byte(b' ');
        } else {
            out.push(b"SSD ");
        }
        out.push(b"?");
    }
}

pub fn git(out: &mut Out, d: Dialect, icons: bool, info: &crate::git::GitInfo) {
    use crate::git::{GitOp, Worktree};
    let br = &info.branch[..usize::from(info.branch_len)];
    if br.is_empty() {
        style(out, d, Color::Red);
        if icons {
            out.push(ICON_GIT);
            out.byte(b' ');
        } else {
            out.push(b"GIT ");
        }
        if let Some(oid) = &info.oid {
            out.byte(b':');
            let hex = b"0123456789abcdef";
            out.byte(hex[usize::from(oid[0] >> 4)]);
            out.byte(hex[usize::from(oid[0] & 0x0F)]);
            out.byte(hex[usize::from(oid[1] >> 4)]);
            out.byte(hex[usize::from(oid[1] & 0x0F)]);
            out.byte(hex[usize::from(oid[2] >> 4)]);
            out.byte(hex[usize::from(oid[2] & 0x0F)]);
            out.byte(hex[usize::from(oid[3] >> 4)]);
        }
        style(out, d, Color::Reset);
    } else {
        style(out, d, Color::Blue);
        if icons {
            out.push(ICON_GIT);
            out.byte(b' ');
        } else {
            out.push(b"GIT ");
        }
        text(out, d, br);
        style(out, d, Color::Reset);

        if info.worktree == Worktree::Modified {
            style(out, d, Color::Yellow);
            out.byte(b'*');
            style(out, d, Color::Reset);
        }

        if let Some(oid) = &info.oid {
            out.byte(b' ');
            style(out, d, Color::Magenta);
            out.byte(b'(');
            let hex = b"0123456789abcdef";
            out.byte(hex[usize::from(oid[0] >> 4)]);
            out.byte(hex[usize::from(oid[0] & 0x0F)]);
            out.byte(hex[usize::from(oid[1] >> 4)]);
            out.byte(hex[usize::from(oid[1] & 0x0F)]);
            out.byte(hex[usize::from(oid[2] >> 4)]);
            out.byte(hex[usize::from(oid[2] & 0x0F)]);
            out.byte(hex[usize::from(oid[3] >> 4)]);
            out.byte(b')');
            style(out, d, Color::Reset);
        }

        if info.worktree == Worktree::Clean {
            style(out, d, Color::Green);
            out.push(" ✓".as_bytes());
            style(out, d, Color::Reset);
        } else if info.worktree == Worktree::Unknown {
            out.push(b" ?");
        }
    }

    match info.op {
        GitOp::None => {}
        GitOp::Merge => {
            style(out, d, Color::Yellow);
            out.push(b"|MERGE");
            style(out, d, Color::Reset);
        }
        GitOp::Rebase => {
            style(out, d, Color::Yellow);
            out.push(b"|REBASE");
            style(out, d, Color::Reset);
        }
        GitOp::CherryPick => {
            style(out, d, Color::Yellow);
            out.push(b"|CHERRY");
            style(out, d, Color::Reset);
        }
        GitOp::Revert => {
            style(out, d, Color::Yellow);
            out.push(b"|REVERT");
            style(out, d, Color::Reset);
        }
        GitOp::Bisect => {
            style(out, d, Color::Yellow);
            out.push(b"|BISECT");
            style(out, d, Color::Reset);
        }
    }
}

pub fn git_err(out: &mut Out, _d: Dialect, icons: bool) {
    if icons {
        out.push(ICON_GIT);
        out.push(b" ?");
    } else {
        out.push(b"GIT ?");
    }
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

    #[test]
    fn git_formatting_icons_and_semantic_colors() {
        use crate::git::{GitInfo, GitOp, Worktree};
        let mut out = Out::new();
        let mut info = GitInfo {
            branch: [0; 128],
            branch_len: 4,
            oid: Some([0x7b, 0x84, 0x96, 0xab, 0, 0, 0, 0]),
            op: GitOp::None,
            worktree: Worktree::Clean,
        };
        info.branch[..4].copy_from_slice(b"main");

        // Plain text mode
        git(&mut out, Dialect::Plain, false, &info);
        assert_eq!(out.as_bytes(), "GIT main (7b8496a) ✓".as_bytes());

        // Ansi with icons (icon in blue scope, sha with parentheses in magenta scope)
        out.rollback(0);
        git(&mut out, Dialect::Ansi, true, &info);
        assert_eq!(
            out.as_bytes(),
            b"\x1b[34m\xef\x90\x98 main\x1b[0m \x1b[35m(7b8496a)\x1b[0m\x1b[32m \xe2\x9c\x93\x1b[0m"
        );

        // Modified worktree
        info.worktree = Worktree::Modified;
        out.rollback(0);
        git(&mut out, Dialect::Plain, true, &info);
        assert_eq!(out.as_bytes(), " main* (7b8496a)".as_bytes());
    }
}
