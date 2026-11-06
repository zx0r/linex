//! `linex` — zero-dependency, zero-allocation telemetry widgets (< 90 lines).

pub mod args;
pub mod cpu;
pub mod fmt;
pub mod git;
pub mod mem;
pub mod ssd;
pub mod state;
pub mod sys;

use std::ffi::OsStr;

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Widget { Cpu, Mem, Ssd, Git }

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Widgets { list: [Widget; 4], len: u8 }

#[rustfmt::skip]
impl Widgets {
    pub const ALL: Self = Self { list: [Widget::Cpu, Widget::Mem, Widget::Ssd, Widget::Git], len: 4 };
    pub const fn empty() -> Self { Self { list: [Widget::Cpu; 4], len: 0 } }
    pub fn push(&mut self, w: Widget) {
        for &item in &self.list[..self.len as usize] { if item == w { return; } }
        if (self.len as usize) < 4 { self.list[self.len as usize] = w; self.len += 1; }
    }
    pub const fn is_empty(&self) -> bool { self.len == 0 }
    pub fn as_slice(&self) -> &[Widget] { &self.list[..self.len as usize] }
}

#[rustfmt::skip]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Config<'a> {
    pub widgets: Widgets, pub dialect: fmt::Dialect, pub icons: bool, pub bytes: bool,
    pub git_detail: bool, pub sep: &'a [u8], pub state_dir: Option<&'a OsStr>,
}

pub fn parse_u64(s: &[u8]) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    s.iter().try_fold(0u64, |acc, &b| {
        let d = b.wrapping_sub(b'0');
        if d > 9 {
            return None;
        }
        acc.checked_mul(10)?.checked_add(u64::from(d))
    })
}

#[rustfmt::skip]
fn cpu_widget(cfg: &Config<'_>, out: &mut fmt::Out) -> bool {
    let Some(curr) = cpu::read() else { fmt::cpu(out, cfg.dialect, cfg.icons, None, false); return false; };
    let Some(file) = cfg.state_dir.and_then(state::open) else { fmt::cpu(out, cfg.dialect, cfg.icons, None, false); return false; };
    let (shown, next) = cpu::step(file.load(), curr);
    if let Some(n) = next { file.store(&n); }
    fmt::cpu(out, cfg.dialect, cfg.icons, shown, true);
    true
}

#[rustfmt::skip]
pub fn render(cfg: &Config<'_>, out: &mut fmt::Out) -> bool {
    let mut healthy = true;
    for &w in cfg.widgets.as_slice() {
        let mark = out.mark();
        if mark != 0 { out.push(cfg.sep); }
        let body = out.mark();
        healthy &= match w {
            Widget::Cpu => cpu_widget(cfg, out),
            Widget::Mem => mem::render(out, cfg.dialect, cfg.icons, cfg.bytes),
            Widget::Ssd => ssd::render(out, cfg.dialect, cfg.icons, cfg.bytes),
            Widget::Git => git::render(out, cfg.dialect, cfg.icons, cfg.git_detail),
        };
        if out.full() { out.rollback(mark); break; }
        if out.mark() == body { out.rollback(mark); }
    }
    healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn lib_tests() {
        assert_eq!(parse_u64(b"0"), Some(0));
        assert_eq!(parse_u64(b"42"), Some(42));
        assert_eq!(parse_u64(b"18446744073709551615"), Some(u64::MAX));
        assert_eq!(parse_u64(b""), None);
        assert_eq!(parse_u64(b"12a"), None);

        let mut w = Widgets::empty();
        w.push(Widget::Git); w.push(Widget::Cpu); w.push(Widget::Git);
        assert_eq!(w.as_slice(), &[Widget::Git, Widget::Cpu]);
    }
}
