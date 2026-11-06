//! Raw Unix FFI bindings and zero-alloc primitives (0 external dependencies).

use std::ffi::{CStr, OsStr};

type CInt = i32;
type CSize = usize;
type CSsize = isize;

pub const O_RDONLY: CInt = 0;
pub const O_RDWR: CInt = 2;
pub const O_CREAT: CInt = if cfg!(target_os = "macos") {
    0x0200
} else {
    0o100
};
pub const O_DIRECTORY: CInt = if cfg!(target_os = "macos") {
    0x0010_0000
} else {
    0o200_000
};
pub const O_CLOEXEC: CInt = if cfg!(target_os = "macos") {
    0x0100_0000
} else {
    0o2_000_000
};
pub const O_NOFOLLOW: CInt = if cfg!(target_os = "macos") {
    0x0100
} else {
    0o400_000
};

pub const S_IFMT: u32 = 0o170_000;
pub const S_IFDIR: u32 = 0o040_000;
pub const S_IFREG: u32 = 0o100_000;

#[cfg(target_os = "macos")]
pub const CTL_HW: CInt = 6;
#[cfg(target_os = "macos")]
pub const HW_MEMSIZE: CInt = 24;
#[cfg(target_os = "macos")]
pub const HOST_CPU_LOAD_INFO: CInt = 3;
#[cfg(target_os = "macos")]
pub const HOST_VM_INFO64: CInt = 4;
#[cfg(target_os = "macos")]
pub const KERN_SUCCESS: CInt = 0;

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Debug)]
pub struct StatFs {
    pub f_bsize: u32,
    pub f_iosize: i32,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: [i32; 2],
    pub f_owner: u32,
    pub f_type: u32,
    pub f_flags: u32,
    pub f_fssubtype: u32,
    pub f_fstypename: [i8; 16],
    pub f_mntonname: [i8; 1024],
    pub f_mntfromname: [i8; 1024],
    pub f_flags_ext: u32,
    pub f_reserved: [u32; 7],
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug)]
pub struct StatVfs {
    pub f_bsize: usize,
    pub f_frsize: usize,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_favail: u64,
    pub f_fsid: usize,
    pub f_flag: usize,
    pub f_namemax: usize,
    pub __f_spare: [CInt; 6],
}

#[repr(C, align(8))]
#[derive(Debug)]
pub struct StatStub {
    pub pad: [u8; 192],
}

unsafe extern "C" {
    #[link_name = "open"]
    fn c_open(p: *const i8, f: CInt, ...) -> CInt;
    #[link_name = "openat"]
    fn c_openat(d: CInt, p: *const i8, f: CInt, ...) -> CInt;
    #[link_name = "close"]
    fn c_close(fd: CInt) -> CInt;
    #[link_name = "read"]
    fn c_read(fd: CInt, b: *mut u8, c: CSize) -> CSsize;
    #[link_name = "pread"]
    fn c_pread(fd: CInt, b: *mut u8, c: CSize, o: i64) -> CSsize;
    #[link_name = "pwrite"]
    fn c_pwrite(fd: CInt, b: *const u8, c: CSize, o: i64) -> CSsize;
    #[link_name = "write"]
    fn c_write(fd: CInt, b: *const u8, c: CSize) -> CSsize;
    #[link_name = "fstat"]
    fn c_fstat(fd: CInt, s: *mut StatStub) -> CInt;
    #[link_name = "fstatat"]
    fn c_fstatat(d: CInt, p: *const i8, s: *mut StatStub, f: CInt) -> CInt;
    #[link_name = "faccessat"]
    fn c_faccessat(d: CInt, p: *const i8, m: CInt, f: CInt) -> CInt;
    #[link_name = "getcwd"]
    fn c_getcwd(b: *mut i8, s: CSize) -> *mut i8;
    #[link_name = "getuid"]
    fn c_getuid() -> u32;
    #[link_name = "mmap"]
    fn c_mmap(a: *mut core::ffi::c_void, l: CSize, p: CInt, f: CInt, fd: CInt, o: i64) -> *mut u8;
    #[link_name = "munmap"]
    fn c_munmap(a: *mut core::ffi::c_void, l: CSize) -> CInt;

    #[cfg(target_os = "macos")]
    pub fn statfs(path: *const i8, buf: *mut StatFs) -> CInt;
    #[cfg(target_os = "macos")]
    pub fn sysctl(
        name: *const CInt,
        namelen: u32,
        oldp: *mut core::ffi::c_void,
        oldlenp: *mut CSize,
        newp: *mut core::ffi::c_void,
        newlen: CSize,
    ) -> CInt;
    #[cfg(target_os = "macos")]
    pub fn mach_host_self() -> u32;
    #[cfg(target_os = "macos")]
    pub fn mach_task_self() -> u32;
    #[cfg(target_os = "macos")]
    pub fn host_statistics(host: u32, flavor: CInt, info: *mut i32, count: *mut u32) -> CInt;
    #[cfg(target_os = "macos")]
    pub fn host_statistics64(host: u32, flavor: CInt, info: *mut i32, count: *mut u32) -> CInt;
    #[cfg(target_os = "macos")]
    pub fn mach_port_deallocate(task: u32, name: u32) -> CInt;

    #[cfg(target_os = "linux")]
    pub fn statvfs(path: *const i8, buf: *mut StatVfs) -> CInt;
}

#[derive(Debug)]
pub struct Fd(pub CInt);
impl Drop for Fd {
    fn drop(&mut self) {
        // SAFETY: closing an open file descriptor owned by this struct.
        unsafe {
            c_close(self.0);
        }
    }
}

impl Fd {
    pub fn open(path: &CStr, flags: CInt) -> Option<Self> {
        // SAFETY: path is a valid null-terminated C string.
        let fd = unsafe { c_open(path.as_ptr(), flags | O_CLOEXEC) };
        (fd >= 0).then_some(Self(fd))
    }
    pub fn openat(&self, name: &CStr, flags: CInt) -> Option<Self> {
        // SAFETY: name is a valid null-terminated C string, self.0 is an open directory fd.
        let fd = unsafe { c_openat(self.0, name.as_ptr(), flags | O_CLOEXEC) };
        (fd >= 0).then_some(Self(fd))
    }
    pub fn faccessat(&self, name: &CStr) -> bool {
        // SAFETY: name is a valid null-terminated C string, self.0 is an open directory fd.
        unsafe { c_faccessat(self.0, name.as_ptr(), 0, 0) == 0 }
    }
}

pub fn read_file<'a>(path: &CStr, buf: &'a mut [u8]) -> Option<&'a [u8]> {
    let fd = Fd::open(path, O_RDONLY)?;
    // SAFETY: fd.0 is an open descriptor, buf is a valid writable buffer.
    let n = unsafe { c_read(fd.0, buf.as_mut_ptr(), buf.len()) };
    (n > 0).then(|| buf.get(..n.cast_unsigned())).flatten()
}

pub fn readat<'a>(dirfd: &Fd, name: &CStr, buf: &'a mut [u8]) -> Option<&'a [u8]> {
    let fd = dirfd.openat(name, O_RDONLY)?;
    // SAFETY: fd.0 is an open descriptor, buf is a valid writable buffer.
    let n = unsafe { c_read(fd.0, buf.as_mut_ptr(), buf.len()) };
    (n > 0).then(|| buf.get(..n.cast_unsigned())).flatten()
}

pub fn pread(fd: &Fd, buf: &mut [u8], offset: i64) -> Option<usize> {
    // SAFETY: fd.0 is an open descriptor, buf is a valid writable buffer.
    let n = unsafe { c_pread(fd.0, buf.as_mut_ptr(), buf.len(), offset) };
    (n >= 0).then_some(n.cast_unsigned())
}

pub fn pwrite(fd: &Fd, buf: &[u8], offset: i64) {
    // SAFETY: fd.0 is an open descriptor, buf is a valid readable buffer.
    unsafe {
        c_pwrite(fd.0, buf.as_ptr(), buf.len(), offset);
    }
}

pub fn write_stdout(buf: &[u8]) -> bool {
    // SAFETY: stdout descriptor 1 is standard, buf is a valid readable buffer.
    let n = unsafe { c_write(1, buf.as_ptr(), buf.len()) };
    n == buf.len().cast_signed()
}

pub fn write_stderr(buf: &[u8]) {
    // SAFETY: stderr descriptor 2 is standard, buf is a valid readable buffer.
    unsafe {
        c_write(2, buf.as_ptr(), buf.len());
    }
}

pub fn getcwd_stack(buf: &mut [u8]) -> Option<usize> {
    // SAFETY: buf is a valid writable byte buffer.
    let p = unsafe { c_getcwd(buf.as_mut_ptr().cast(), buf.len()) };
    (!p.is_null())
        .then(|| buf.iter().position(|&b| b == 0))
        .flatten()
}

pub fn lstat_mode(path: &CStr) -> Option<u32> {
    let mut st = core::mem::MaybeUninit::<StatStub>::uninit();
    // AT_FDCWD = -2 (Linux) / -2 (macOS)
    // SAFETY: path is a valid null-terminated C string, st pointer is valid for StatStub write.
    let rc = unsafe {
        c_fstatat(
            -2,
            path.as_ptr(),
            st.as_mut_ptr(),
            0x0020, /* AT_SYMLINK_NOFOLLOW */
        )
    };
    if rc != 0 {
        return None;
    }
    // SAFETY: c_fstatat succeeded and initialized the buffer.
    let st = unsafe { st.assume_init() };
    // Offset of st_mode: on macOS it's at offset 4 (u16), on Linux at offset 16 (u32).
    let mode = if cfg!(target_os = "macos") {
        let m = u16::from_ne_bytes([st.pad[4], st.pad[5]]);
        u32::from(m)
    } else {
        u32::from_ne_bytes([st.pad[16], st.pad[17], st.pad[18], st.pad[19]])
    };
    Some(mode)
}

#[derive(Debug)]
pub struct Map {
    ptr: *mut u8,
    len: usize,
}

impl Map {
    pub fn new(fd: &Fd) -> Option<Self> {
        let mut st = core::mem::MaybeUninit::<StatStub>::uninit();
        // SAFETY: fd.0 is an open descriptor, st pointer is valid for StatStub write.
        let stat_res = unsafe { c_fstat(fd.0, st.as_mut_ptr()) };
        if stat_res != 0 {
            return None;
        }
        // SAFETY: c_fstat succeeded and initialized the buffer.
        let st = unsafe { st.assume_init() };
        // st_size offset: macOS is 96 (i64), Linux x86_64 is 48 (i64).
        let len: i64 = if cfg!(target_os = "macos") {
            i64::from_ne_bytes([
                st.pad[96],
                st.pad[97],
                st.pad[98],
                st.pad[99],
                st.pad[100],
                st.pad[101],
                st.pad[102],
                st.pad[103],
            ])
        } else {
            i64::from_ne_bytes([
                st.pad[48], st.pad[49], st.pad[50], st.pad[51], st.pad[52], st.pad[53], st.pad[54],
                st.pad[55],
            ])
        };
        let ulen = usize::try_from(len).ok()?;
        if ulen == 0 {
            return None;
        }
        // SAFETY: mapping `ulen` bytes of open fd.0 with PROT_READ and MAP_PRIVATE.
        let map_ptr = unsafe {
            c_mmap(
                core::ptr::null_mut(),
                ulen,
                1, /* PROT_READ */
                2, /* MAP_PRIVATE */
                fd.0,
                0,
            )
        };
        if map_ptr.is_null() || map_ptr == usize::MAX as *mut u8 {
            return None;
        }
        Some(Self {
            ptr: map_ptr,
            len: ulen,
        })
    }

    pub const fn as_bytes(&self) -> &[u8] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            // SAFETY: self.ptr is non-null and valid for reads of self.len bytes while self is live.
            unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
        }
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            // SAFETY: unmapping memory mapped in Map::new with identical pointer and len.
            unsafe {
                c_munmap(self.ptr.cast(), self.len);
            }
        }
    }
}

pub fn map_at(dirfd: &Fd, name: &CStr) -> Option<Map> {
    let fd = dirfd.openat(name, O_RDONLY)?;
    Map::new(&fd)
}

pub fn open_state(dir: &OsStr) -> Option<Fd> {
    use std::os::unix::ffi::OsStrExt;
    let mut path = [0u8; 512];
    let d = dir.as_bytes();
    // SAFETY: getuid is a safe query of the current process user ID.
    let uid = unsafe { c_getuid() };
    let mut ubuf = [0u8; 20];
    let ustr = itoa(u64::from(uid), &mut ubuf);
    let total = d.len() + 1 + 7 + ustr.len() + 5;
    if total > path.len() {
        return None;
    }
    let mut p = 0;
    path[p..p + d.len()].copy_from_slice(d);
    p += d.len();
    path[p] = b'/';
    p += 1;
    path[p..p + 7].copy_from_slice(b"statty-");
    p += 7;
    path[p..p + ustr.len()].copy_from_slice(ustr);
    p += ustr.len();
    path[p..p + 5].copy_from_slice(b".cpu\0");
    p += 5;
    // SAFETY: path contains valid ASCII bytes ending with an explicit null terminator at p-1.
    let cpath = unsafe { CStr::from_bytes_with_nul_unchecked(&path[..p]) };
    let flags = O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC;
    // SAFETY: cpath is a valid null-terminated C string and mode 0600 is valid.
    let fd = unsafe { c_open(cpath.as_ptr(), flags, 0o600_u32) };
    (fd >= 0).then_some(Fd(fd))
}

fn itoa(mut v: u64, buf: &mut [u8; 20]) -> &[u8] {
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    &buf[i..]
}

pub fn state_dir() -> Option<std::ffi::OsString> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .or_else(|| Some("/tmp".into()))
}
