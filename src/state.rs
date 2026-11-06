//! Persistent state file I/O (< 30 lines).

use crate::cpu::{self, State};
use crate::sys::{self, Fd};
use std::ffi::OsStr;

#[derive(Debug)]
pub struct StateFile(Fd);

pub fn open(dir: &OsStr) -> Option<StateFile> {
    sys::open_state(dir).map(StateFile)
}

impl StateFile {
    pub fn load(&self) -> Option<State> {
        let mut buf = [0u8; cpu::STATE_LEN];
        let n = sys::pread(&self.0, &mut buf, 0)?;
        if n != cpu::STATE_LEN {
            return None;
        }
        cpu::decode(buf)
    }

    pub fn store(&self, s: &State) {
        let data = cpu::encode(s);
        sys::pwrite(&self.0, &data, 0);
    }
}
