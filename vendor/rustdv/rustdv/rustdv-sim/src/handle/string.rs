//! String values with scheduled and immediate writes.

use rustdv_gpi as gpi;

use crate::{
    handle::{HandleBase, ValueError},
    phase,
};

/// A string variable. Embedded NUL bytes are rejected by both write paths.
#[derive(Copy, Clone)]
pub struct StringHandle {
    pub(super) raw: gpi::StringHandle,
}

impl From<gpi::StringHandle> for StringHandle {
    fn from(raw: gpi::StringHandle) -> Self {
        StringHandle { raw }
    }
}

impl HandleBase for StringHandle {
    fn name(&self) -> String {
        self.raw.name()
    }

    fn full_name(&self) -> String {
        self.raw.full_name()
    }
}

impl StringHandle {
    pub fn get(&self) -> Result<String, ValueError> {
        self.raw.get()
    }

    /// Buffer an owned copy until ReadWrite; the last write wins.
    pub fn set(&self, value: &str) -> Result<(), ValueError> {
        phase::deny_write_in_read_only(self.raw);
        if value.contains('\0') {
            return Err(ValueError::InteriorNul);
        }
        phase::schedule(self.raw, phase::WriteVal::String(value.into()));
        Ok(())
    }

    pub fn set_now(&self, value: &str) -> Result<(), ValueError> {
        phase::deny_write_in_read_only(self.raw);
        self.raw.set_now(value)
    }
}
