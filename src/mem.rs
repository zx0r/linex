//! RAM probe: total and available bytes (< 60 lines).

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MemInfo { pub total: u64, pub available: u64 }

#[cfg(target_os = "linux")]
#[rustfmt::skip]
pub fn read() -> Option<MemInfo> {
    let mut buf = [0u8; 1024];
    let bytes = crate::sys::read_file(c"/proc/meminfo", &mut buf)?;
    let mut total = None;
    let mut avail = None;
    for line in bytes.split(|&b| b == b'\n') {
        if let Some(r) = line.strip_prefix(b"MemTotal:") { total = parse_kb(r); }
        else if let Some(r) = line.strip_prefix(b"MemAvailable:") { avail = parse_kb(r); }
    }
    Some(MemInfo { total: total? * 1024, available: avail? * 1024 })
}

#[cfg(target_os = "linux")]
#[rustfmt::skip]
fn parse_kb(s: &[u8]) -> Option<u64> {
    let t = s.iter().position(|&b| b != b' ' && b != b'\t')?;
    let d = s[t..].iter().position(|&b| !b.is_ascii_digit()).unwrap_or(s.len() - t);
    crate::parse_u64(&s[t..t + d])
}

#[cfg(target_os = "macos")]
#[rustfmt::skip]
pub fn read() -> Option<MemInfo> {
    use core::mem::size_of;
    let mut total: u64 = 0;
    let mut size = size_of::<u64>();
    let mib = [crate::sys::CTL_HW, crate::sys::HW_MEMSIZE];
    // SAFETY: mib points to CTL_HW/HW_MEMSIZE, size and total pointers are valid.
    let rc = unsafe { crate::sys::sysctl(mib.as_ptr(), 2, (&raw mut total).cast(), &raw mut size, core::ptr::null_mut(), 0) };
    if rc != 0 { return None; }
    let mut vm_info = [0u32; 38];
    let mut count = 38u32;
    // SAFETY: acquire Mach host port for querying VM stats.
    let host = unsafe { crate::sys::mach_host_self() };
    // SAFETY: vm_info buffer is large enough for HOST_VM_INFO64.
    let kr = unsafe { crate::sys::host_statistics64(host, crate::sys::HOST_VM_INFO64, vm_info.as_mut_ptr().cast(), &raw mut count) };
    // SAFETY: acquire self task port.
    let task = unsafe { crate::sys::mach_task_self() };
    // SAFETY: release host port.
    unsafe { crate::sys::mach_port_deallocate(task, host); }
    if kr != crate::sys::KERN_SUCCESS { return None; }
    // SAFETY: getpagesize() is a standard POSIX C query with no side effects.
    let page_size = u64::try_from(unsafe { crate::sys::getpagesize() }).unwrap_or(16384);
    let used_pages = u64::from(vm_info[35].saturating_sub(vm_info[22])) + u64::from(vm_info[3]) + u64::from(vm_info[32]);
    let used = (used_pages * page_size).min(total);
    Some(MemInfo { total, available: total.saturating_sub(used) })
}

#[rustfmt::skip]
pub fn render(out: &mut crate::fmt::Out, d: crate::fmt::Dialect, icons: bool, bytes: bool) -> bool {
    if let Some(m) = read() {
        crate::fmt::mem(out, d, icons, bytes, m.total.saturating_sub(m.available), m.total);
        true
    } else {
        crate::fmt::mem_err(out, icons);
        false
    }
}
