//! Raw Unix FFI bindings and zero-alloc primitives (0 external dependencies).

use std::ffi::{CStr, OsStr};

type CInt = i32;
type CSize = usize;
type CSsize = isize;

pub const O_RDONLY: CInt = 0;
pub const O_RDWR: CInt = 2;
#[rustfmt::skip] pub const O_CREAT: CInt = if cfg!(target_os = "macos") { 0x0200 } else { 0o100 };
#[rustfmt::skip] pub const O_DIRECTORY: CInt = if cfg!(target_os = "macos") { 0x0010_0000 } else { 0o200_000 };
#[rustfmt::skip] pub const O_CLOEXEC: CInt = if cfg!(target_os = "macos") { 0x0100_0000 } else { 0o2_000_000 };
#[rustfmt::skip] pub const O_NOFOLLOW: CInt = if cfg!(target_os = "macos") { 0x0100 } else { 0o400_000 };

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

#[rustfmt::skip]
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Debug)]
pub struct StatFs {
    pub f_bsize: u32, pub f_iosize: i32, pub f_blocks: u64, pub f_bfree: u64, pub f_bavail: u64,
    pub f_files: u64, pub f_ffree: u64, pub f_fsid: [i32; 2], pub f_owner: u32, pub f_type: u32,
    pub f_flags: u32, pub f_fssubtype: u32, pub f_fstypename: [i8; 16], pub f_mntonname: [i8; 1024],
    pub f_mntfromname: [i8; 1024], pub f_flags_ext: u32, pub f_reserved: [u32; 7],
}

#[rustfmt::skip]
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug)]
pub struct StatVfs {
    pub f_bsize: usize, pub f_frsize: usize, pub f_blocks: u64, pub f_bfree: u64, pub f_bavail: u64,
    pub f_files: u64, pub f_ffree: u64, pub f_favail: u64, pub f_fsid: usize, pub f_flag: usize,
    pub f_namemax: usize, pub __f_spare: [CInt; 6],
}

#[repr(C, align(8))]
#[derive(Debug)]
pub struct StatStub {
    pub pad: [u8; 192],
}

#[rustfmt::skip]
unsafe extern "C" {
    #[link_name = "open"] fn c_open(p: *const i8, f: CInt, ...) -> CInt;
    #[link_name = "openat"] fn c_openat(d: CInt, p: *const i8, f: CInt, ...) -> CInt;
    #[link_name = "close"] fn c_close(fd: CInt) -> CInt;
    #[link_name = "read"] fn c_read(fd: CInt, b: *mut u8, c: CSize) -> CSsize;
    #[link_name = "pread"] fn c_pread(fd: CInt, b: *mut u8, c: CSize, o: i64) -> CSsize;
    #[link_name = "pwrite"] fn c_pwrite(fd: CInt, b: *const u8, c: CSize, o: i64) -> CSsize;
    #[link_name = "write"] fn c_write(fd: CInt, b: *const u8, c: CSize) -> CSsize;
    #[link_name = "fstatat"] fn c_fstatat(d: CInt, p: *const i8, s: *mut StatStub, f: CInt) -> CInt;
    #[link_name = "faccessat"] fn c_faccessat(d: CInt, p: *const i8, m: CInt, f: CInt) -> CInt;
    #[link_name = "getcwd"] fn c_getcwd(b: *mut i8, s: CSize) -> *mut i8;
    #[link_name = "getuid"] fn c_getuid() -> u32;
    pub fn getpagesize() -> CInt;

    #[cfg(target_os = "macos")] pub fn statfs(path: *const i8, buf: *mut StatFs) -> CInt;
    #[cfg(target_os = "macos")] pub fn sysctl(n: *const CInt, nl: u32, op: *mut core::ffi::c_void, ol: *mut CSize, np: *mut core::ffi::c_void, nl2: CSize) -> CInt;
    #[cfg(target_os = "macos")] pub fn mach_host_self() -> u32;
    #[cfg(target_os = "macos")] pub fn mach_task_self() -> u32;
    #[cfg(target_os = "macos")] pub fn host_statistics(host: u32, flavor: CInt, info: *mut i32, count: *mut u32) -> CInt;
    #[cfg(target_os = "macos")] pub fn host_statistics64(host: u32, flavor: CInt, info: *mut i32, count: *mut u32) -> CInt;
    #[cfg(target_os = "macos")] pub fn mach_port_deallocate(task: u32, name: u32) -> CInt;

    #[cfg(target_os = "linux")] pub fn statvfs(path: *const i8, buf: *mut StatVfs) -> CInt;
}

#[derive(Debug)]
pub struct Fd(pub CInt);
impl Drop for Fd {
    fn drop(&mut self) {
        // SAFETY: closing open file descriptor.
        unsafe {
            c_close(self.0);
        }
    }
}

#[rustfmt::skip]
impl Fd {
    pub fn open(path: &CStr, flags: CInt) -> Option<Self> {
        // SAFETY: path is null-terminated C string.
        let fd = unsafe { c_open(path.as_ptr(), flags | O_CLOEXEC) };
        (fd >= 0).then_some(Self(fd))
    }
    pub fn openat(&self, name: &CStr, flags: CInt) -> Option<Self> {
        // SAFETY: name is null-terminated, self.0 is open dir fd.
        let fd = unsafe { c_openat(self.0, name.as_ptr(), flags | O_CLOEXEC) };
        (fd >= 0).then_some(Self(fd))
    }
    pub fn faccessat(&self, name: &CStr) -> bool {
        // SAFETY: name is null-terminated, self.0 is open dir fd.
        unsafe { c_faccessat(self.0, name.as_ptr(), 0, 0) == 0 }
    }
}

#[rustfmt::skip]
pub fn read_file<'a>(path: &CStr, buf: &'a mut [u8]) -> Option<&'a [u8]> {
    let fd = Fd::open(path, O_RDONLY)?;
    // SAFETY: fd.0 is open, buf is valid for writing.
    let n = unsafe { c_read(fd.0, buf.as_mut_ptr(), buf.len()) };
    (n > 0).then(|| buf.get(..n.cast_unsigned())).flatten()
}

#[rustfmt::skip]
pub fn readat<'a>(dirfd: &Fd, name: &CStr, buf: &'a mut [u8]) -> Option<&'a [u8]> {
    let fd = dirfd.openat(name, O_RDONLY)?;
    // SAFETY: fd.0 is open, buf is valid for writing.
    let n = unsafe { c_read(fd.0, buf.as_mut_ptr(), buf.len()) };
    (n > 0).then(|| buf.get(..n.cast_unsigned())).flatten()
}

#[rustfmt::skip]
pub fn pread(fd: &Fd, buf: &mut [u8], offset: i64) -> Option<usize> {
    // SAFETY: fd.0 is open, buf is valid for writing.
    let n = unsafe { c_pread(fd.0, buf.as_mut_ptr(), buf.len(), offset) };
    (n >= 0).then_some(n.cast_unsigned())
}

#[rustfmt::skip]
pub fn pwrite(fd: &Fd, buf: &[u8], offset: i64) {
    // SAFETY: fd.0 is open, buf is valid for reading.
    unsafe { c_pwrite(fd.0, buf.as_ptr(), buf.len(), offset); }
}

#[rustfmt::skip]
pub fn write_stdout(buf: &[u8]) -> bool {
    // SAFETY: fd 1 is stdout, buf is valid for reading.
    let n = unsafe { c_write(1, buf.as_ptr(), buf.len()) };
    n == buf.len().cast_signed()
}

#[rustfmt::skip]
pub fn write_stderr(buf: &[u8]) {
    // SAFETY: fd 2 is stderr, buf is valid for reading.
    unsafe { c_write(2, buf.as_ptr(), buf.len()); }
}

#[rustfmt::skip]
pub fn getcwd_stack(buf: &mut [u8]) -> Option<usize> {
    // SAFETY: buf is valid for writing.
    let p = unsafe { c_getcwd(buf.as_mut_ptr().cast(), buf.len()) };
    (!p.is_null()).then(|| buf.iter().position(|&b| b == 0)).flatten()
}

#[rustfmt::skip]
pub fn lstat_mode(path: &CStr) -> Option<u32> {
    let mut st = core::mem::MaybeUninit::<StatStub>::uninit();
    // SAFETY: path is null-terminated, st pointer is valid for writing.
    let rc = unsafe { c_fstatat(-2, path.as_ptr(), st.as_mut_ptr(), 0x0020) };
    if rc != 0 { return None; }
    // SAFETY: c_fstatat succeeded.
    let st = unsafe { st.assume_init() };
    let mode = if cfg!(target_os = "macos") {
        u32::from(u16::from_ne_bytes([st.pad[4], st.pad[5]]))
    } else {
        u32::from_ne_bytes([st.pad[16], st.pad[17], st.pad[18], st.pad[19]])
    };
    Some(mode)
}

#[rustfmt::skip]
pub fn mtime_at(dir: &Fd, path: &CStr) -> Option<i64> {
    let mut st = core::mem::MaybeUninit::<StatStub>::uninit();
    // SAFETY: dir.0 is open, path is null-terminated, st is valid for writing.
    let rc = unsafe { c_fstatat(dir.0, path.as_ptr(), st.as_mut_ptr(), 0) };
    if rc != 0 { return None; }
    // SAFETY: c_fstatat succeeded.
    let st = unsafe { st.assume_init() };
    let mtime = if cfg!(target_os = "macos") {
        i64::from_ne_bytes([st.pad[48], st.pad[49], st.pad[50], st.pad[51], st.pad[52], st.pad[53], st.pad[54], st.pad[55]])
    } else {
        i64::from_ne_bytes([st.pad[88], st.pad[89], st.pad[90], st.pad[91], st.pad[92], st.pad[93], st.pad[94], st.pad[95]])
    };
    Some(mtime)
}

#[rustfmt::skip]
pub fn open_state(dir: &OsStr) -> Option<Fd> {
    // SAFETY: querying user id.
    let uid = unsafe { c_getuid() };
    let path = std::path::Path::new(dir).join(format!("statty-{uid}.cpu\0"));
    let cpath = CStr::from_bytes_with_nul(path.as_os_str().as_encoded_bytes()).ok()?;
    let flags = O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC;
    // SAFETY: cpath is valid null-terminated C string, mode 0600 is valid.
    let fd = unsafe { c_open(cpath.as_ptr(), flags, 0o600_u32) };
    (fd >= 0).then_some(Fd(fd))
}

#[rustfmt::skip]
pub fn state_dir() -> Option<std::ffi::OsString> {
    std::env::var_os("XDG_RUNTIME_DIR").or_else(|| std::env::var_os("TMPDIR")).or_else(|| Some("/tmp".into()))
}
