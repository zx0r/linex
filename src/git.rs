//! Git workspace probe: discovery, branch, short oid, op-state, worktree status (< 140 lines).

use crate::sys::{self, Fd};
use std::ffi::CStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum GitOp {
    None,
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Worktree {
    Clean,
    Modified,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GitError;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GitStatus {
    pub branch: [u8; 128],
    pub branch_len: u8,
    pub is_detached: bool,
    pub oid: Option<[u8; 8]>,
    pub ahead: u16,
    pub behind: u16,
    pub is_gone: bool,
    pub op: GitOp,
    pub staged: u16,
    pub unstaged: u16,
    pub untracked: u16,
    pub conflicted: u16,
    pub stashed: u16,
    pub worktree: Worktree,
}

pub type GitInfo = GitStatus;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Head<'a> {
    Branch(&'a [u8]),
    Detached(&'a [u8]),
}

pub fn parse_head(buf: &[u8]) -> Option<Head<'_>> {
    let line = buf.split(|&b| b == b'\n' || b == b'\r').next()?;
    if let Some(name) = line.strip_prefix(b"ref: refs/heads/") {
        Some(Head::Branch(name))
    } else if (line.len() == 40 || line.len() == 64) && line.iter().all(u8::is_ascii_hexdigit) {
        Some(Head::Detached(line))
    } else {
        None
    }
}

pub fn parse_gitfile(buf: &[u8]) -> Option<&[u8]> {
    let line = buf.split(|&b| b == b'\n' || b == b'\r').next()?;
    line.strip_prefix(b"gitdir: ")
}

pub fn find_packed_ref<'a>(map: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    for line in map.split(|&b| b == b'\n') {
        if line.starts_with(b"#") || line.starts_with(b"^") {
            continue;
        }
        if let Some(sp) = line.iter().position(|&b| b == b' ')
            && &line[sp + 1..] == name
        {
            return Some(&line[..sp]);
        }
    }
    None
}

pub fn parse_index_header(buf: &[u8]) -> Option<(u32, u32)> {
    if buf.len() < 12 || !buf.starts_with(b"DIRC") {
        return None;
    }
    let ver = u32::from_be_bytes(buf.get(4..8)?.try_into().ok()?);
    let count = u32::from_be_bytes(buf.get(8..12)?.try_into().ok()?);
    Some((ver, count))
}

fn detect_op(git_fd: &Fd) -> GitOp {
    if git_fd.faccessat(c"rebase-merge") || git_fd.faccessat(c"rebase-apply") {
        GitOp::Rebase
    } else if git_fd.faccessat(c"MERGE_HEAD") {
        GitOp::Merge
    } else if git_fd.faccessat(c"CHERRY_PICK_HEAD") {
        GitOp::CherryPick
    } else if git_fd.faccessat(c"REVERT_HEAD") {
        GitOp::Revert
    } else if git_fd.faccessat(c"BISECT_LOG") {
        GitOp::Bisect
    } else {
        GitOp::None
    }
}

#[allow(clippy::naive_bytecount, reason = "zero-dep bounded 1KB buffer")]
fn count_stash(git_fd: &Fd) -> u16 {
    let mut buf = [0u8; 1024];
    if let Some(data) = sys::readat(git_fd, c"logs/refs/stash", &mut buf) {
        let count = data.iter().filter(|&&b| b == b'\n').count();
        u16::try_from(count).unwrap_or(u16::MAX)
    } else {
        0
    }
}

fn trim_ascii(b: &[u8]) -> &[u8] {
    let start = b
        .iter()
        .position(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .unwrap_or(b.len());
    let end = b
        .iter()
        .rposition(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .map_or(start, |p| p + 1);
    &b[start..end]
}

fn resolve_upstream(
    git_fd: &Fd,
    common_fd: &Fd,
    branch_name: &[u8],
    local_oid: Option<[u8; 8]>,
) -> (u16, u16, bool) {
    let mut cbuf = [0u8; 4096];
    let Some(conf) = sys::readat(common_fd, c"config", &mut cbuf) else {
        return (0, 0, false);
    };

    let mut needle = [0u8; 140];
    let pref = b"[branch \"";
    needle[..pref.len()].copy_from_slice(pref);
    needle[pref.len()..pref.len() + branch_name.len()].copy_from_slice(branch_name);
    needle[pref.len() + branch_name.len()..pref.len() + branch_name.len() + 2]
        .copy_from_slice(b"\"]");
    let target = &needle[..pref.len() + branch_name.len() + 2];

    let mut remote: Option<&[u8]> = None;
    let mut merge: Option<&[u8]> = None;

    if let Some(pos) = conf.windows(target.len()).position(|w| w == target) {
        let sec = &conf[pos + target.len()..];
        for line in sec.split(|&b| b == b'\n') {
            let tr = trim_ascii(line);
            if tr.starts_with(b"[") {
                break;
            }
            if let Some(val) = tr.strip_prefix(b"remote =") {
                remote = Some(trim_ascii(val));
            } else if let Some(val) = tr.strip_prefix(b"remote=") {
                remote = Some(trim_ascii(val));
            } else if let Some(val) = tr.strip_prefix(b"merge =") {
                merge = Some(trim_ascii(val));
            } else if let Some(val) = tr.strip_prefix(b"merge=") {
                merge = Some(trim_ascii(val));
            }
        }
    }

    let (Some(rem), Some(mrg)) = (remote, merge) else {
        return (0, 0, false);
    };

    let branch_short = mrg.strip_prefix(b"refs/heads/").unwrap_or(mrg);
    let mut ref_path = [0u8; 256];
    let rpref = b"refs/remotes/";
    ref_path[..rpref.len()].copy_from_slice(rpref);
    let mut cur = rpref.len();
    ref_path[cur..cur + rem.len()].copy_from_slice(rem);
    cur += rem.len();
    ref_path[cur] = b'/';
    cur += 1;
    ref_path[cur..cur + branch_short.len()].copy_from_slice(branch_short);
    cur += branch_short.len();
    ref_path[cur] = 0;

    // SAFETY: ref_path contains explicit null terminator at cur.
    let rcstr = unsafe { CStr::from_bytes_with_nul_unchecked(&ref_path[..=cur]) };
    let mut obuf = [0u8; 64];
    let mut remote_oid = None;

    if let Some(raw) = sys::readat(common_fd, rcstr, &mut obuf) {
        let mut oid = [0u8; 8];
        hex_to_bin(&raw[..raw.len().min(16)], &mut oid);
        remote_oid = Some(oid);
    } else if let Some(map) = sys::map_at(common_fd, c"packed-refs")
        && let Some(hex) = find_packed_ref(map.as_bytes(), &ref_path[..cur])
    {
        let mut oid = [0u8; 8];
        hex_to_bin(&hex[..hex.len().min(16)], &mut oid);
        remote_oid = Some(oid);
    }

    match (local_oid, remote_oid) {
        (Some(l), Some(r)) => {
            if l == r {
                (0, 0, false)
            } else {
                let mut log_buf = [0u8; 4096];
                let ahead_detected =
                    if let Some(log) = sys::readat(git_fd, c"logs/HEAD", &mut log_buf) {
                        let mut r_hex = [0u8; 16];
                        bin_to_hex(&r, &mut r_hex);
                        log.windows(16).any(|w| w == r_hex)
                    } else {
                        false
                    };
                if ahead_detected {
                    (1, 0, false)
                } else {
                    (0, 1, false)
                }
            }
        }
        (Some(_), None) => (0, 0, true),
        _ => (0, 0, false),
    }
}

fn discover_root(cwd: &mut [u8], cwd_len: usize) -> Option<usize> {
    let mut root_len = None;
    let mut k = cwd_len;
    for _ in 0..64 {
        if k + 6 >= cwd.len() {
            break;
        }
        cwd[k..k + 6].copy_from_slice(b"/.git\0");
        // SAFETY: cwd contains valid bytes with an explicit null terminator at k + 5.
        let probe = unsafe { CStr::from_bytes_with_nul_unchecked(&cwd[..k + 6]) };
        if let Some(m) = sys::lstat_mode(probe)
            && (m & sys::S_IFMT == sys::S_IFDIR || m & sys::S_IFMT == sys::S_IFREG)
        {
            root_len = Some(k);
            break;
        }
        if k == 0 {
            break;
        }
        k = cwd[..k].iter().rposition(|&b| b == b'/').unwrap_or(0);
    }
    root_len
}

/// Probes the git workspace starting from the current working directory.
///
/// # Errors
///
/// Returns `GitError` if directory probing or repository I/O encounters a fatal failure.
pub fn read() -> Result<Option<GitStatus>, GitError> {
    let mut cwd = [0u8; 1024];
    let cwd_len = sys::getcwd_stack(&mut cwd).ok_or(GitError)?;
    let Some(r_len) = discover_root(&mut cwd, cwd_len) else {
        return Ok(None);
    };

    cwd[r_len] = 0;
    // SAFETY: cwd[r_len] was explicitly set to 0 (null terminator).
    let r_path = unsafe { CStr::from_bytes_with_nul_unchecked(&cwd[..=r_len]) };
    let root_fd = Fd::open(r_path, sys::O_RDONLY | sys::O_DIRECTORY).ok_or(GitError)?;

    // Open .git
    cwd[r_len..r_len + 6].copy_from_slice(b"/.git\0");
    // SAFETY: bytes at r_len..r_len+6 end with an explicit null terminator.
    let g_path = unsafe { CStr::from_bytes_with_nul_unchecked(&cwd[..r_len + 6]) };
    let g_mode = sys::lstat_mode(g_path).ok_or(GitError)?;
    let git_fd = if g_mode & sys::S_IFMT == sys::S_IFREG {
        let mut gbuf = [0u8; 512];
        let content = sys::readat(&root_fd, c".git", &mut gbuf).ok_or(GitError)?;
        let target = parse_gitfile(content).ok_or(GitError)?;
        let mut tbuf = [0u8; 512];
        tbuf[..target.len()].copy_from_slice(target);
        tbuf[target.len()] = 0;
        // SAFETY: tbuf contains target bytes ending with explicit null terminator at target.len().
        let tcstr = unsafe { CStr::from_bytes_with_nul_unchecked(&tbuf[..=target.len()]) };
        if target.starts_with(b"/") {
            Fd::open(tcstr, sys::O_RDONLY | sys::O_DIRECTORY).ok_or(GitError)?
        } else {
            root_fd
                .openat(tcstr, sys::O_RDONLY | sys::O_DIRECTORY)
                .ok_or(GitError)?
        }
    } else {
        root_fd
            .openat(c".git", sys::O_RDONLY | sys::O_DIRECTORY)
            .ok_or(GitError)?
    };

    // Commondir
    let mut cbuf = [0u8; 256];
    let common_fd_cell;
    let mut common_fd = &git_fd;
    if let Some(cd) = sys::readat(&git_fd, c"commondir", &mut cbuf) {
        let trim = cd.split(|&b| b == b'\n' || b == b'\r').next().unwrap_or(cd);
        let mut tbuf = [0u8; 256];
        tbuf[..trim.len()].copy_from_slice(trim);
        tbuf[trim.len()] = 0;
        // SAFETY: tbuf contains trim bytes ending with explicit null terminator at trim.len().
        let tc = unsafe { CStr::from_bytes_with_nul_unchecked(&tbuf[..=trim.len()]) };
        if let Some(cfd) = git_fd.openat(tc, sys::O_RDONLY | sys::O_DIRECTORY) {
            common_fd_cell = cfd;
            common_fd = &common_fd_cell;
        }
    }

    // HEAD
    let mut hbuf = [0u8; 256];
    let head = sys::readat(&git_fd, c"HEAD", &mut hbuf).ok_or(GitError)?;
    let parsed_head = parse_head(head);

    let mut status = GitStatus {
        branch: [0; 128],
        branch_len: 0,
        is_detached: false,
        oid: None,
        ahead: 0,
        behind: 0,
        is_gone: false,
        op: detect_op(&git_fd),
        staged: 0,
        unstaged: 0,
        untracked: 0,
        conflicted: 0,
        stashed: count_stash(&git_fd),
        worktree: Worktree::Unknown,
    };

    match parsed_head {
        Some(Head::Branch(name)) => {
            let len = name.len().min(128);
            status.branch[..len].copy_from_slice(&name[..len]);
            status.branch_len = u8::try_from(len).unwrap_or(128);

            // Resolve OID: refs/heads/<name>
            let mut rbuf = [0u8; 256];
            let pref = b"refs/heads/";
            rbuf[..pref.len()].copy_from_slice(pref);
            rbuf[pref.len()..pref.len() + name.len()].copy_from_slice(name);
            rbuf[pref.len() + name.len()] = 0;
            // SAFETY: rbuf contains pref + name with null terminator at pref.len() + name.len().
            let rcstr =
                unsafe { CStr::from_bytes_with_nul_unchecked(&rbuf[..=(pref.len() + name.len())]) };

            let mut obuf = [0u8; 64];
            if let Some(raw) = sys::readat(common_fd, rcstr, &mut obuf) {
                let mut oid = [0u8; 8];
                hex_to_bin(&raw[..raw.len().min(16)], &mut oid);
                status.oid = Some(oid);
            } else if let Some(map) = sys::map_at(common_fd, c"packed-refs") {
                let refname = &rbuf[..pref.len() + name.len()];
                if let Some(hex) = find_packed_ref(map.as_bytes(), refname) {
                    let mut oid = [0u8; 8];
                    hex_to_bin(&hex[..hex.len().min(16)], &mut oid);
                    status.oid = Some(oid);
                }
            }

            let (a, b, gone) = resolve_upstream(&git_fd, common_fd, name, status.oid);
            status.ahead = a;
            status.behind = b;
            status.is_gone = gone;
        }
        Some(Head::Detached(hex)) => {
            status.is_detached = true;
            let mut oid = [0u8; 8];
            hex_to_bin(&hex[..hex.len().min(16)], &mut oid);
            status.oid = Some(oid);
        }
        None => {}
    }

    // Worktree state: quick index check + mtime delta
    if let Some(map) = sys::map_at(&git_fd, c"index")
        && let Some((ver, _count)) = parse_index_header(map.as_bytes())
        && (ver == 2 || ver == 3 || ver == 4)
    {
        let idx_mtime = sys::mtime_at(&git_fd, c"index");
        let head_mtime = sys::mtime_at(&git_fd, c"HEAD");
        if let (Some(idx), Some(head)) = (idx_mtime, head_mtime) {
            if idx > head {
                status.worktree = Worktree::Modified;
                status.unstaged = 1;
            } else {
                status.worktree = Worktree::Clean;
            }
        } else {
            status.worktree = Worktree::Clean;
        }
    }

    Ok(Some(status))
}

fn hex_to_bin(hex: &[u8], out: &mut [u8]) {
    let mut i = 0;
    let mut j = 0;
    while i + 1 < hex.len() && j < out.len() {
        let h = match hex[i] {
            b'0'..=b'9' => hex[i] - b'0',
            b'a'..=b'f' => hex[i] - b'a' + 10,
            _ => 0,
        };
        let l = match hex[i + 1] {
            b'0'..=b'9' => hex[i + 1] - b'0',
            b'a'..=b'f' => hex[i + 1] - b'a' + 10,
            _ => 0,
        };
        out[j] = (h << 4) | l;
        i += 2;
        j += 1;
    }
}

fn bin_to_hex(bin: &[u8], out: &mut [u8]) {
    let hex = b"0123456789abcdef";
    for (i, &b) in bin.iter().enumerate() {
        if i * 2 + 1 < out.len() {
            out[i * 2] = hex[usize::from(b >> 4)];
            out[i * 2 + 1] = hex[usize::from(b & 0x0F)];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_head_branch() {
        let b = b"ref: refs/heads/main\n";
        assert_eq!(parse_head(b), Some(Head::Branch(b"main")));
    }

    #[test]
    fn parse_head_detached() {
        let b = b"0123456789abcdef0123456789abcdef01234567\n";
        assert_eq!(parse_head(b), Some(Head::Detached(&b[..40])));
    }

    #[test]
    fn parse_gitfile_target() {
        let b = b"gitdir: ../.git/worktrees/feat\n";
        assert_eq!(parse_gitfile(b), Some(&b"../.git/worktrees/feat"[..]));
    }

    #[test]
    fn packed_ref_lookup() {
        let map = b"# comment\n^peeled\n0123456789abcdef0123456789abcdef01234567 refs/heads/main\n";
        assert_eq!(
            find_packed_ref(map, b"refs/heads/main"),
            Some(&b"0123456789abcdef0123456789abcdef01234567"[..])
        );
        assert_eq!(find_packed_ref(map, b"refs/heads/missing"), None);
    }

    #[test]
    fn index_header() {
        let mut data = [0u8; 12];
        data[..4].copy_from_slice(b"DIRC");
        data[4..8].copy_from_slice(&2u32.to_be_bytes());
        data[8..12].copy_from_slice(&42u32.to_be_bytes());
        assert_eq!(parse_index_header(&data), Some((2, 42)));
    }
}
