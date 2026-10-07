//! Real values with scheduled and immediate writes.

use rustdv_gpi as gpi;

use crate::{
    handle::{HandleBase, ValueError},
    phase,
};

/// A real variable with scheduled and immediate writes.
#[derive(Copy, Clone)]
pub struct RealHandle {
    pub(super) raw: gpi::RealHandle,
}

impl From<gpi::RealHandle> for RealHandle {
    fn from(raw: gpi::RealHandle) -> Self {
        RealHandle { raw }
    }
}

impl HandleBase for RealHandle {
    fn name(&self) -> String {
        self.raw.name()
    }

    fn full_name(&self) -> String {
        self.raw.full_name()
    }
}

impl RealHandle {
    pub fn get(&self) -> Result<f64, ValueError> {
        self.raw.get()
    }
    /// Buffer a write until ReadWrite; the last write to this handle wins.
    pub fn set(&self, value: f64) {
        phase::schedule(self.raw, phase::WriteVal::Real(value));
    }
    pub fn set_now(&self, value: f64) {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_now(value);
    }
}
