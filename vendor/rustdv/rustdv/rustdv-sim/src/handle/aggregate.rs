//! Typed member access for structs and unions.

use rustdv_gpi as gpi;

use crate::handle::{HandleBase, HandleChildren, HandleError, SimHandle};

/// An unpacked struct or union accessed through typed members.
#[derive(Copy, Clone)]
pub struct AggregateHandle {
    pub(super) raw: gpi::AggregateHandle,
}

impl From<gpi::AggregateHandle> for AggregateHandle {
    fn from(raw: gpi::AggregateHandle) -> Self {
        AggregateHandle { raw }
    }
}

impl HandleChildren for AggregateHandle {
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

impl HandleBase for AggregateHandle {
    fn name(&self) -> String {
        self.raw.name()
    }

    fn full_name(&self) -> String {
        self.raw.full_name()
    }
}
