//! Filesystem space probe (< 40 lines).

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SsdInfo { pub total: u64, pub avail: u64 }

#[rustfmt::skip]
pub const fn usage(blocks: u64, bfree: u64, bavail: u64, frsize: u64) -> SsdInfo {
    let used = blocks.saturating_sub(bfree);
    SsdInfo { total: used.saturating_add(bavail).saturating_mul(frsize), avail: bavail.saturating_mul(frsize) }
}

#[cfg(target_os = "macos")]
#[rustfmt::skip]
pub fn read() -> Option<SsdInfo> {
    let mut st = core::mem::MaybeUninit::<crate::sys::StatFs>::uninit();
    // SAFETY: c"." is valid null-terminated, st pointer is valid for writing StatFs.
    let rc = unsafe { crate::sys::statfs(c".".as_ptr(), st.as_mut_ptr()) };
    if rc != 0 { return None; }
    // SAFETY: statfs initialized st.
    let st = unsafe { st.assume_init() };
    Some(usage(st.f_blocks, st.f_bfree, st.f_bavail, u64::from(st.f_bsize)))
}

#[cfg(target_os = "linux")]
#[rustfmt::skip]
pub fn read() -> Option<SsdInfo> {
    let mut st = core::mem::MaybeUninit::<crate::sys::StatVfs>::uninit();
    // SAFETY: c"." is valid null-terminated, st pointer is valid for writing StatVfs.
    let rc = unsafe { crate::sys::statvfs(c".".as_ptr(), st.as_mut_ptr()) };
    if rc != 0 { return None; }
    // SAFETY: statvfs initialized st.
    let st = unsafe { st.assume_init() };
    let frsize = if st.f_frsize != 0 { st.f_frsize } else { st.f_bsize };
    Some(usage(st.f_blocks, st.f_bfree, st.f_bavail, frsize as u64))
}

pub const ICON_SSD: &[u8] = "\u{f02ca}".as_bytes();

const C_GREEN: u8 = 84;
const C_YELLOW: u8 = 227;
const C_RED: u8 = 203;

#[inline]
pub const fn color_for_ssd(bp: u16) -> u8 {
    match bp {
        0..=7500 => C_GREEN,
        7501..=9000 => C_YELLOW,
        _ => C_RED,
    }
}

#[rustfmt::skip]
pub fn render(out: &mut crate::fmt::Out, d: crate::fmt::Dialect, icons: bool, bytes: bool) -> bool {
    if let Some(s) = read() {
        let used = s.total.saturating_sub(s.avail);
        let total = s.total;
        let bp = if total > 0 { u16::try_from((u128::from(used) * 10_000) / u128::from(total)).unwrap_or(10_000) } else { 0 };
        crate::fmt::color(out, d, color_for_ssd(bp));
        if icons { out.push(ICON_SSD); out.byte(b' '); } else { out.push(b"SSD "); }
        if bytes { crate::fmt::human_bytes(out, used); out.byte(b'/'); crate::fmt::human_bytes(out, total); } else { crate::fmt::percent(out, bp); }
        crate::fmt::reset(out, d);
        true
    } else {
        if icons { out.push(ICON_SSD); out.byte(b' '); } else { out.push(b"SSD "); }
        out.push(b"?");
        false
    }
}
