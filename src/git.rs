//! Zero-overhead Git VFS probe with full porcelain v2 telemetry and SWR cache.

use crate::fmt::{self, Out};
use crate::sys::{self, Fd};
use std::ffi::CStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

pub const ICON_GIT: &[u8] = "\u{f418}".as_bytes();

#[derive(Default)]
struct Metrics {
    ahead: u16,
    behind: u16,
    staged: u16,
    unstaged: u16,
    untracked: u16,
    conflicted: u16,
}

#[rustfmt::skip]
fn parse_u16(b: &[u8]) -> u16 {
    let mut v = 0u16;
    for &c in b {
        if !c.is_ascii_digit() { break; }
        v = v.saturating_mul(10).saturating_add(u16::from(c - b'0'));
    }
    v
}

#[rustfmt::skip]
fn probe_root(cwd: &mut [u8; 1024], target_path: Option<&Path>) -> Option<(usize, usize)> {
    let mut cwd_len = match target_path {
        Some(p) => {
            let b = p.as_os_str().as_encoded_bytes();
            if b.is_empty() || b.len() >= 1000 {
                sys::getcwd_stack(cwd)?
            } else {
                cwd[..b.len()].copy_from_slice(b);
                b.len()
            }
        }
        None => sys::getcwd_stack(cwd)?,
    };
    while cwd_len > 1 && cwd[cwd_len - 1] == b'/' { cwd_len -= 1; }
    let mut k = cwd_len;
    for _ in 0..32 {
        if k + 6 >= cwd.len() || k == 0 { break; }
        cwd[k..k + 6].copy_from_slice(b"/.git\0");
        // SAFETY: probe is explicitly null-terminated at k + 5.
        let probe = unsafe { CStr::from_bytes_with_nul_unchecked(&cwd[..k + 6]) };
        if let Some(m) = sys::lstat_mode(probe) && (m & sys::S_IFMT == sys::S_IFDIR || m & sys::S_IFMT == sys::S_IFREG) {
            return Some((k, cwd_len));
        }
        k = cwd[..k].iter().rposition(|&b| b == b'/').unwrap_or(0);
    }
    None
}

#[rustfmt::skip]
fn resolve_sha(git_fd: &Fd, line: &[u8], sha: &mut [u8; 7]) -> bool {
    if let Some(r) = line.strip_prefix(b"ref: ") {
        let mut r_buf = [0u8; 128]; let len = r.len().min(127); r_buf[..len].copy_from_slice(&r[..len]);
        // SAFETY: r_buf is explicitly null-terminated at len.
        let c_ref = unsafe { CStr::from_bytes_with_nul_unchecked(&r_buf[..=len]) };
        let mut b = [0u8; 64];
        if let Some(r_slice) = sys::readat(git_fd, c_ref, &mut b) && r_slice.len() >= 7 { sha.copy_from_slice(&r_slice[..7]); return true; }
        let mut pbuf = [0u8; 2048];
        if let Some(p) = sys::readat(git_fd, c"packed-refs", &mut pbuf) {
            for pl in p.split(|&c| c == b'\n') {
                if pl.ends_with(r) && pl.len() >= 7 && !pl.starts_with(b"#") { sha.copy_from_slice(&pl[..7]); return true; }
            }
        }
    } else if line.len() >= 7 { sha.copy_from_slice(&line[..7]); return true; }
    false
}

#[rustfmt::skip]
fn parse_cache(buf: &[u8]) -> (Metrics, u16) {
    let mut m = Metrics::default();
    let mut stashed = 0u16;
    for line in buf.split(|&b| b == b'\n') {
        if let Some(ab) = line.strip_prefix(b"# branch.ab +") {
            if let Some(p) = ab.iter().position(|&b| b == b' ') {
                m.ahead = parse_u16(&ab[..p]);
                if let Some(b) = ab[p..].strip_prefix(b" -") { m.behind = parse_u16(b); }
            }
        } else if let Some(st) = line.strip_prefix(b"# branch.stash ") { stashed = parse_u16(st); }
        else if line.starts_with(b"? ") { m.untracked = m.untracked.saturating_add(1); }
        else if line.starts_with(b"u ") { m.conflicted = m.conflicted.saturating_add(1); }
        else if (line.starts_with(b"1 ") || line.starts_with(b"2 ")) && line.len() > 3 {
            if line[2] != b'.' { m.staged = m.staged.saturating_add(1); }
            if line[3] != b'.' { m.unstaged = m.unstaged.saturating_add(1); }
        }
    }
    (m, stashed)
}

#[rustfmt::skip]
fn render_counters(out: &mut Out, d: fmt::Dialect, m: &Metrics, stashed: u16) {
    let list: [(u16, &[u8], fmt::Color); 5] = [
        (m.staged, "\u{25cf}".as_bytes(), fmt::Color::Green), (m.unstaged, "\u{271a}".as_bytes(), fmt::Color::Yellow),
        (m.untracked, b"?", fmt::Color::Blue), (m.conflicted, "\u{2716}".as_bytes(), fmt::Color::Red),
        (stashed, "\u{2691}".as_bytes(), fmt::Color::Pink),
    ];
    for (v, sym, col) in list {
        if v > 0 { out.byte(b' '); fmt::style(out, d, col); out.push(sym); fmt::u64(out, u64::from(v)); fmt::style(out, d, fmt::Color::Reset); }
    }
}

#[rustfmt::skip]
pub fn render(out: &mut Out, d: fmt::Dialect, icons: bool, detail: bool) -> bool {
    render_at(out, d, icons, detail, None)
}

#[rustfmt::skip]
pub fn render_at(out: &mut Out, d: fmt::Dialect, icons: bool, _detail: bool, path: Option<&Path>) -> bool {
    let mut cwd = [0u8; 1024];
    let Some((r_len, _)) = probe_root(&mut cwd, path) else { return true; };
    cwd[r_len..r_len + 6].copy_from_slice(b"/.git\0");
    // SAFETY: g_path is explicitly null-terminated at r_len + 5.
    let g_path = unsafe { CStr::from_bytes_with_nul_unchecked(&cwd[..r_len + 6]) };
    let Some(git_fd) = Fd::open(g_path, sys::O_RDONLY | sys::O_DIRECTORY) else { return false; };
    let mut hbuf = [0u8; 256];
    let Some(head) = sys::readat(&git_fd, c"HEAD", &mut hbuf) else { return false; };
    let line = head.split(|&b| b == b'\n' || b == b'\r').next().unwrap_or(head);

    let op: &[u8] = if git_fd.faccessat(c"rebase-merge/interactive") { b"(rebase-i)" }
    else if git_fd.faccessat(c"rebase-merge") { b"(rebase-m)" }
    else if git_fd.faccessat(c"rebase-apply/rebasing") { b"(rebase)" }
    else if git_fd.faccessat(c"rebase-apply/applying") { b"(am)" }
    else if git_fd.faccessat(c"rebase-apply") { b"(am/rebase)" }
    else if git_fd.faccessat(c"MERGE_HEAD") { b"(merge)" }
    else if git_fd.faccessat(c"CHERRY_PICK_HEAD") { b"(cherry-pick)" }
    else if git_fd.faccessat(c"REVERT_HEAD") { b"(revert)" }
    else if git_fd.faccessat(c"BISECT_LOG") { b"(bisect)" } else { b"" };

    let idx_m = sys::mtime_at(&git_fd, c"index").unwrap_or(0);
    let head_m = sys::mtime_at(&git_fd, c"HEAD").unwrap_or(0);
    let is_dirty = idx_m > head_m;

    let mut sha = [0u8; 7];
    let has_sha = resolve_sha(&git_fd, line, &mut sha);

    let mut hasher = DefaultHasher::new();
    cwd[..r_len + 5].hash(&mut hasher);
    let cache_path = std::env::temp_dir().join(format!("linex_git_{:x}.cache", hasher.finish()));
    let max_mtime = idx_m.max(head_m).max(sys::mtime_at(&git_fd, c"FETCH_HEAD").unwrap_or(0));
    let c_mtime = std::fs::metadata(&cache_path).ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |x| i64::try_from(x.as_secs()).unwrap_or(0));
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |x| i64::try_from(x.as_secs()).unwrap_or(0));

    let (m, mut stashed) = if c_mtime == 0 || (now - c_mtime) >= 2 || c_mtime < max_mtime {
        let start_dir = Path::new(std::str::from_utf8(&cwd[..r_len]).unwrap_or("."));
        if let Ok(out) = std::process::Command::new("git").args(["status", "--porcelain=v2", "--branch", "--show-stash"]).current_dir(start_dir).output() {
            let _ = std::fs::write(&cache_path, &out.stdout);
            parse_cache(&out.stdout)
        } else if let Ok(c) = std::fs::read(&cache_path) { parse_cache(&c) } else { (Metrics::default(), 0) }
    } else if let Ok(c) = std::fs::read(&cache_path) { parse_cache(&c) } else { (Metrics::default(), 0) };

    let mut stbuf = [0u8; 1024];
    if stashed == 0 && let Some(b) = sys::readat(&git_fd, c"logs/refs/stash", &mut stbuf) {
        for &c in b { if c == b'\n' { stashed = stashed.saturating_add(1); } }
    }

    fmt::style(out, d, fmt::Color::Blue);
    fmt::bold(out, d);
    if icons { out.push(ICON_GIT); out.byte(b' '); } else { out.push(b"GIT "); }
    if let Some(name) = line.strip_prefix(b"ref: refs/heads/") { fmt::text(out, d, name); } else { fmt::style(out, d, fmt::Color::Red); out.push(b"detached"); }
    fmt::style(out, d, fmt::Color::Reset);

    if is_dirty || m.unstaged > 0 { fmt::style(out, d, fmt::Color::Yellow); out.byte(b'*'); fmt::style(out, d, fmt::Color::Reset); }
    if !op.is_empty() { out.byte(b' '); fmt::style(out, d, fmt::Color::Yellow); out.push(op); fmt::style(out, d, fmt::Color::Reset); }

    if has_sha { out.byte(b' '); fmt::style(out, d, fmt::Color::Magenta); out.byte(b'('); out.push(&sha); out.byte(b')'); fmt::style(out, d, fmt::Color::Reset); }
    if m.ahead > 0 { out.byte(b' '); fmt::style(out, d, fmt::Color::Cyan); out.push("⇡".as_bytes()); fmt::u64(out, u64::from(m.ahead)); fmt::style(out, d, fmt::Color::Reset); }
    if m.behind > 0 { out.byte(b' '); fmt::style(out, d, fmt::Color::Yellow); out.push("⇣".as_bytes()); fmt::u64(out, u64::from(m.behind)); fmt::style(out, d, fmt::Color::Reset); }
    render_counters(out, d, &m, stashed);
    if !is_dirty && m.staged == 0 && m.unstaged == 0 && m.untracked == 0 && m.conflicted == 0 { fmt::style(out, d, fmt::Color::Green); out.push(" ✓".as_bytes()); fmt::style(out, d, fmt::Color::Reset); }
    out.byte(b' ');
    true
}
