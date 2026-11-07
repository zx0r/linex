//! Zero-overhead system uptime probe.
//!
//! Pure POSIX / Darwin C ABI without heap allocations.
//! macOS: `sysctl [CTL_KERN, KERN_BOOTTIME]`
//! Linux: `/proc/uptime`

#[cfg(target_os = "macos")]
pub fn uptime_secs() -> Option<u64> {
    use crate::sys;
    use core::mem::size_of;

    #[repr(C)]
    struct Timeval {
        tv_sec: i64,
        tv_usec: i32,
        _pad: i32,
    }

    let mut mib = [sys::CTL_KERN, sys::KERN_BOOTTIME];
    let mut boot = Timeval {
        tv_sec: 0,
        tv_usec: 0,
        _pad: 0,
    };
    let mut len = size_of::<Timeval>();

    // SAFETY: mib points to valid 2-element array, boot and len are valid pointers for output.
    let ret = unsafe {
        sys::c_sysctl(
            mib.as_mut_ptr(),
            2,
            (&raw mut boot).cast::<core::ffi::c_void>(),
            &raw mut len,
            core::ptr::null_mut(),
            0,
        )
    };

    if ret != 0 || boot.tv_sec <= 0 {
        return None;
    }

    // SAFETY: c_time accepts null pointer to return current epoch seconds.
    let now = unsafe { sys::c_time(core::ptr::null_mut()) };
    if now <= 0 || now < boot.tv_sec {
        return None;
    }

    u64::try_from(now - boot.tv_sec).ok()
}

#[cfg(target_os = "linux")]
pub fn uptime_secs() -> Option<u64> {
    use crate::sys::{self, Fd};

    let fd = Fd::open(c"/proc/uptime", sys::O_RDONLY)?;
    let mut buf = [0u8; 64];
    let b = sys::read_buf(&fd, &mut buf)?;

    let mut sec = 0u64;
    for &c in b {
        if c == b'.' || c == b' ' {
            break;
        }
        if c.is_ascii_digit() {
            sec = sec.saturating_mul(10).saturating_add(u64::from(c - b'0'));
        }
    }
    Some(sec)
}

pub const ICON_UPTIME: &[u8] = "\u{f176}".as_bytes();

pub fn render(out: &mut crate::fmt::Out, d: crate::fmt::Dialect, icons: bool) -> bool {
    if let Some(secs) = uptime_secs() {
        crate::fmt::reset(out, d);
        if icons {
            out.push(ICON_UPTIME);
        } else {
            out.push(b"UP ");
        }
        if secs >= 365 * 86400 {
            crate::fmt::u64(out, secs / (365 * 86400));
            out.byte(b'y');
        } else if secs >= 30 * 86400 {
            crate::fmt::u64(out, secs / (30 * 86400));
            out.byte(b'm');
        } else if secs >= 7 * 86400 {
            crate::fmt::u64(out, secs / (7 * 86400));
            out.byte(b'w');
        } else if secs >= 86400 {
            crate::fmt::u64(out, secs / 86400);
            out.byte(b'd');
        } else if secs >= 3600 {
            crate::fmt::u64(out, secs / 3600);
            out.byte(b'h');
        } else {
            crate::fmt::u64(out, (secs / 60).max(1));
            out.byte(b'm');
        }
        crate::fmt::reset(out, d);
        true
    } else {
        if icons {
            out.push(ICON_UPTIME);
        } else {
            out.push(b"UP ");
        }
        out.push(b"?");
        false
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn uptime_secs() -> Option<u64> {
    None
}
