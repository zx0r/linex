//! Filesystem space probe (< 40 lines).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SsdInfo {
    pub total: u64,
    pub avail: u64,
}

pub const fn usage(blocks: u64, bfree: u64, bavail: u64, frsize: u64) -> SsdInfo {
    let used = blocks.saturating_sub(bfree);
    SsdInfo {
        total: used.saturating_add(bavail).saturating_mul(frsize),
        avail: bavail.saturating_mul(frsize),
    }
}

#[cfg(target_os = "macos")]
pub fn read() -> Option<SsdInfo> {
    let mut st = core::mem::MaybeUninit::<crate::sys::StatFs>::uninit();
    // SAFETY: c"." is a valid null-terminated C string, st pointer is valid for writing StatFs.
    let rc = unsafe { crate::sys::statfs(c".".as_ptr(), st.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statfs succeeded and fully initialized st.
    let st = unsafe { st.assume_init() };
    Some(usage(
        st.f_blocks,
        st.f_bfree,
        st.f_bavail,
        u64::from(st.f_bsize),
    ))
}

#[cfg(target_os = "linux")]
pub fn read() -> Option<SsdInfo> {
    let mut st = core::mem::MaybeUninit::<crate::sys::StatVfs>::uninit();
    // SAFETY: c"." is a valid null-terminated C string, st pointer is valid for writing StatVfs.
    let rc = unsafe { crate::sys::statvfs(c".".as_ptr(), st.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statvfs succeeded and fully initialized st.
    let st = unsafe { st.assume_init() };
    let frsize = if st.f_frsize != 0 {
        st.f_frsize
    } else {
        st.f_bsize
    };
    Some(usage(st.f_blocks, st.f_bfree, st.f_bavail, frsize as u64))
}
