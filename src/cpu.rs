//! CPU probe: delta accounting, state codec, platform counter read (< 90 lines).

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ticks { pub busy: u64, pub idle: u64 }

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct State { pub pct: u16, pub ticks: Ticks }

pub const NO_PCT: u16 = u16::MAX;
const MIN_DELTA: u64 = 100;
const WRAP: u64 = if cfg!(target_os = "macos") {
    u32::MAX as u64
} else {
    u64::MAX
};

#[rustfmt::skip]
pub fn step(prev: Option<State>, curr: Ticks) -> (Option<u16>, Option<State>) {
    let seed = State { pct: NO_PCT, ticks: curr };
    let Some(p) = prev else { return (None, Some(seed)); };
    let busy = curr.busy.wrapping_sub(p.ticks.busy) & WRAP;
    let idle = curr.idle.wrapping_sub(p.ticks.idle) & WRAP;
    if busy > WRAP / 2 || idle > WRAP / 2 { return (None, Some(seed)); }
    let total = busy + idle;
    let shown = (p.pct != NO_PCT).then_some(p.pct);
    if total < MIN_DELTA { return (shown, None); }
    let pct = u16::try_from((u128::from(busy) * 10_000) / u128::from(total)).unwrap_or(10_000);
    (Some(pct), Some(State { pct, ticks: curr }))
}

pub const STATE_LEN: usize = 24;
const MAGIC: [u8; 4] = *b"STT\x03";

#[rustfmt::skip]
pub fn encode(s: &State) -> [u8; STATE_LEN] {
    let mut b = [0u8; STATE_LEN];
    b[0..4].copy_from_slice(&MAGIC);
    b[4..6].copy_from_slice(&s.pct.to_ne_bytes());
    b[8..16].copy_from_slice(&s.ticks.busy.to_ne_bytes());
    b[16..24].copy_from_slice(&s.ticks.idle.to_ne_bytes());
    b
}

#[rustfmt::skip]
pub fn decode(b: [u8; STATE_LEN]) -> Option<State> {
    if b[0..4] != MAGIC { return None; }
    let pct = u16::from_ne_bytes([b[4], b[5]]);
    if pct > 10_000 && pct != NO_PCT { return None; }
    let busy = u64::from_ne_bytes([b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]]);
    let idle = u64::from_ne_bytes([b[16], b[17], b[18], b[19], b[20], b[21], b[22], b[23]]);
    Some(State { pct, ticks: Ticks { busy, idle } })
}

#[cfg(target_os = "linux")]
#[rustfmt::skip]
pub fn read() -> Option<Ticks> {
    let mut buf = [0u8; 1024];
    let bytes = crate::sys::read_file(c"/proc/stat", &mut buf)?;
    let line = bytes.strip_prefix(b"cpu ")?.split(|&b| b == b'\n').next()?;
    let mut it = line.split(|&b| b == b' ').filter(|s| !s.is_empty());
    let mut f = [0u64; 8];
    for slot in &mut f {
        if let Some(tok) = it.next() { *slot = crate::parse_u64(tok)?; } else { break; }
    }
    let [u, n, s, i, io, irq, sirq, st] = f;
    Some(Ticks { busy: u + n + s + irq + sirq + st, idle: i + io })
}

#[cfg(target_os = "macos")]
#[rustfmt::skip]
pub fn read() -> Option<Ticks> {
    let mut info = [0u32; 4];
    let mut count = 4u32;
    // SAFETY: querying host self port.
    let host = unsafe { crate::sys::mach_host_self() };
    // SAFETY: info buffer has 4 natural_t integers matching HOST_CPU_LOAD_INFO count.
    let kr = unsafe { crate::sys::host_statistics(host, crate::sys::HOST_CPU_LOAD_INFO, info.as_mut_ptr().cast(), &raw mut count) };
    // SAFETY: querying current task self port.
    let task = unsafe { crate::sys::mach_task_self() };
    // SAFETY: deallocating mach host port.
    unsafe { crate::sys::mach_port_deallocate(task, host); }
    if kr != crate::sys::KERN_SUCCESS { return None; }
    let [user, sys, idle, nice] = info;
    Some(Ticks { busy: u64::from(user.wrapping_add(sys).wrapping_add(nice)), idle: u64::from(idle) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn codec_and_step() {
        let s = State { pct: 4200, ticks: Ticks { busy: 1234, idle: 5678 } };
        assert_eq!(decode(encode(&s)), Some(s));
        let (_, save) = step(None, Ticks { busy: 100, idle: 100 });
        let s = save.unwrap();
        assert_eq!(step(Some(s), Ticks { busy: 110, idle: 110 }), (None, None));
        assert_eq!(step(Some(s), Ticks { busy: 160, idle: 140 }), (Some(6000), Some(State { pct: 6000, ticks: Ticks { busy: 160, idle: 140 } })));
    }
}
