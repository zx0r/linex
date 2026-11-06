//! CPU probe: delta accounting, state codec, platform counter read (< 90 lines).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ticks {
    pub busy: u64,
    pub idle: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct State {
    pub pct: u16,
    pub ticks: Ticks,
}

pub const NO_PCT: u16 = u16::MAX;
const MIN_DELTA: u64 = 100;
const WRAP: u64 = if cfg!(target_os = "macos") {
    u32::MAX as u64
} else {
    u64::MAX
};

pub fn step(prev: Option<State>, curr: Ticks) -> (Option<u16>, Option<State>) {
    let seed = State {
        pct: NO_PCT,
        ticks: curr,
    };
    let Some(p) = prev else {
        return (None, Some(seed));
    };
    let busy = curr.busy.wrapping_sub(p.ticks.busy) & WRAP;
    let idle = curr.idle.wrapping_sub(p.ticks.idle) & WRAP;
    if busy > WRAP / 2 || idle > WRAP / 2 {
        return (None, Some(seed));
    }
    let total = busy + idle;
    let shown = (p.pct != NO_PCT).then_some(p.pct);
    if total < MIN_DELTA {
        return (shown, None);
    }
    let pct = u16::try_from((u128::from(busy) * 10_000) / u128::from(total)).unwrap_or(10_000);
    (Some(pct), Some(State { pct, ticks: curr }))
}

pub const STATE_LEN: usize = 24;
const MAGIC: [u8; 4] = *b"STT\x03";

pub const fn encode(s: &State) -> [u8; STATE_LEN] {
    let [m0, m1, m2, m3] = MAGIC;
    let [p0, p1] = s.pct.to_ne_bytes();
    let [b0, b1, b2, b3, b4, b5, b6, b7] = s.ticks.busy.to_ne_bytes();
    let [i0, i1, i2, i3, i4, i5, i6, i7] = s.ticks.idle.to_ne_bytes();
    [
        m0, m1, m2, m3, p0, p1, 0, 0, b0, b1, b2, b3, b4, b5, b6, b7, i0, i1, i2, i3, i4, i5, i6,
        i7,
    ]
}

pub const fn decode(b: [u8; STATE_LEN]) -> Option<State> {
    let [
        m0,
        m1,
        m2,
        m3,
        p0,
        p1,
        _,
        _,
        b0,
        b1,
        b2,
        b3,
        b4,
        b5,
        b6,
        b7,
        i0,
        i1,
        i2,
        i3,
        i4,
        i5,
        i6,
        i7,
    ] = b;
    if m0 != MAGIC[0] || m1 != MAGIC[1] || m2 != MAGIC[2] || m3 != MAGIC[3] {
        return None;
    }
    let pct = u16::from_ne_bytes([p0, p1]);
    if pct > 10_000 && pct != NO_PCT {
        return None;
    }
    Some(State {
        pct,
        ticks: Ticks {
            busy: u64::from_ne_bytes([b0, b1, b2, b3, b4, b5, b6, b7]),
            idle: u64::from_ne_bytes([i0, i1, i2, i3, i4, i5, i6, i7]),
        },
    })
}

#[cfg(target_os = "linux")]
pub fn read() -> Option<Ticks> {
    let mut buf = [0u8; 1024];
    let bytes = crate::sys::read_file(c"/proc/stat", &mut buf)?;
    let line = bytes.strip_prefix(b"cpu ")?.split(|&b| b == b'\n').next()?;
    let mut it = line.split(|&b| b == b' ').filter(|s| !s.is_empty());
    let mut f = [0u64; 8];
    for slot in &mut f {
        if let Some(tok) = it.next() {
            *slot = crate::parse_u64(tok)?;
        } else {
            break;
        }
    }
    let [u, n, s, i, io, irq, sirq, st] = f;
    Some(Ticks {
        busy: u + n + s + irq + sirq + st,
        idle: i + io,
    })
}

#[cfg(target_os = "macos")]
pub fn read() -> Option<Ticks> {
    let mut info = [0u32; 4];
    let mut count = 4u32;
    // SAFETY: mach_host_self returns host port for host statistics querying.
    let host = unsafe { crate::sys::mach_host_self() };
    // SAFETY: info buffer has 4 natural_t integers matching HOST_CPU_LOAD_INFO count.
    let kr = unsafe {
        crate::sys::host_statistics(
            host,
            crate::sys::HOST_CPU_LOAD_INFO,
            info.as_mut_ptr().cast(),
            &raw mut count,
        )
    };
    // SAFETY: mach_task_self returns current task port.
    let task = unsafe { crate::sys::mach_task_self() };
    // SAFETY: deallocate host send right.
    unsafe { crate::sys::mach_port_deallocate(task, host) };
    if kr != crate::sys::KERN_SUCCESS {
        return None;
    }
    let [user, sys, idle, nice] = info;
    Some(Ticks {
        busy: u64::from(user.wrapping_add(sys).wrapping_add(nice)),
        idle: u64::from(idle),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_roundtrip() {
        let s = State {
            pct: 4200,
            ticks: Ticks {
                busy: 1234,
                idle: 5678,
            },
        };
        let enc = encode(&s);
        let dec = decode(enc).unwrap();
        assert_eq!(dec, s);
    }

    #[test]
    fn step_state_machine() {
        let (shown, save) = step(
            None,
            Ticks {
                busy: 100,
                idle: 100,
            },
        );
        assert_eq!(shown, None); // cold start
        let s = save.unwrap();
        // Accumulate < 100 delta
        let (shown2, save2) = step(
            Some(s),
            Ticks {
                busy: 110,
                idle: 110,
            },
        );
        assert_eq!(shown2, None);
        assert!(save2.is_none());
        // Trigger >= 100 delta
        let (shown3, save3) = step(
            Some(s),
            Ticks {
                busy: 160,
                idle: 140,
            },
        );
        assert_eq!(shown3, Some(6000)); // 60% busy
        assert_eq!(save3.unwrap().pct, 6000);
    }
}
