//! Module lookup and typed child access.

use rustdv_gpi as gpi;

use crate::handle::{HandleBase, HandleChildren, HandleError, SimHandle};

/// A module/scope handle: `dut.child("name")?` (OQ-6 dynamic-first lean).
#[derive(Copy, Clone)]
pub struct HierarchyHandle {
    pub(super) raw: gpi::HierarchyHandle,
}

impl From<gpi::HierarchyHandle> for HierarchyHandle {
    fn from(raw: gpi::HierarchyHandle) -> Self {
        HierarchyHandle { raw }
    }
}

impl HandleBase for HierarchyHandle {
    fn name(&self) -> String {
        self.raw.name()
    }

    fn full_name(&self) -> String {
        self.raw.full_name()
    }
}

impl HandleChildren for HierarchyHandle {
    fn child(&self, name: &str) -> Result<SimHandle, HandleError> {
        Ok(SimHandle::wrap(self.raw.child(name)?))
    }

    fn children(&self) -> Vec<SimHandle> {
        self.raw
            .children()
            .into_iter()
            .map(SimHandle::wrap)
            .collect()
    }
}

/// The `RUSTDV_TOP` module, or the first top-level module when it is unset.
pub fn top_module() -> Result<HierarchyHandle, HandleError> {
    Ok(HierarchyHandle::from(gpi::top_module()?))
}

impl std::fmt::Debug for HierarchyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HierarchyHandle(\"{}\")", self.full_name())
    }
}

impl HierarchyHandle {
    /// A handle to nothing, for unit tests that need a `RustdvCtx` but never
    /// touch the DUT. Any VPI call through it reaches `rustdv-vpi-stubs`,
    /// which panics — so a test that *does* touch the DUT fails loudly rather
    /// than reading garbage, and its author learns it belongs in a `sim-*`
    /// case instead.
    pub fn null_for_test() -> HierarchyHandle {
        HierarchyHandle {
            raw: gpi::HierarchyHandle::null_for_test(),
        }
    }
}
