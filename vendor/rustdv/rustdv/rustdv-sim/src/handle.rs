//! Typed DUT handles with scheduled-write semantics and edge methods —
//! the user-facing layer over rustdv-gpi (design-doc mapping rows 14,
//! 19–22). `set()` is buffered and applied at the next ReadWrite phase;
//! `set_now()` is the immediate variant.

use rustdv_gpi as gpi;
pub use rustdv_gpi::{BigUint, HandleError, Logic, LogicArray, ValueError};

pub mod aggregate;
pub mod hierarchy;
pub mod logic;
pub mod real;
pub mod string;

pub use aggregate::AggregateHandle;
pub use hierarchy::{HierarchyHandle, top_module};
pub use logic::LogicHandle;
pub use real::RealHandle;
pub use string::StringHandle;

use crate::triggers::Edge;

#[derive(Copy, Clone)]
pub enum SimHandle {
    Hierarchy(HierarchyHandle),
    Logic(LogicHandle),
    Real(RealHandle),
    String(StringHandle),
    Struct(AggregateHandle),
    Union(AggregateHandle),
    Other,
}

impl SimHandle {
    fn wrap(h: gpi::AnyHandle) -> SimHandle {
        match h {
            gpi::AnyHandle::Hierarchy(raw) => SimHandle::Hierarchy(HierarchyHandle { raw }),
            gpi::AnyHandle::Logic(raw) => SimHandle::Logic(LogicHandle { raw }),
            gpi::AnyHandle::Real(raw) => SimHandle::Real(RealHandle { raw }),
            gpi::AnyHandle::String(raw) => SimHandle::String(StringHandle { raw }),
            gpi::AnyHandle::Struct(raw) => SimHandle::Struct(AggregateHandle { raw }),
            gpi::AnyHandle::Union(raw) => SimHandle::Union(AggregateHandle { raw }),
            gpi::AnyHandle::Other(_) => SimHandle::Other,
        }
    }

    fn wrong_kind(self, expected: &'static str) -> HandleError {
        let (name, actual) = match self {
            Self::Hierarchy(h) => (h.full_name(), "module"),
            Self::Logic(h) => (h.full_name(), "signal"),
            Self::Real(h) => (h.full_name(), "real"),
            Self::String(h) => (h.full_name(), "string"),
            Self::Struct(h) => (h.full_name(), "struct"),
            Self::Union(h) => (h.full_name(), "union"),
            // Other carries no underlying object metadata.
            Self::Other => ("<unknown>".into(), "unsupported simulator object"),
        };
        HandleError::WrongKind {
            name,
            expected,
            actual: actual.into(),
        }
    }

    pub fn as_hierarchy(self) -> Result<HierarchyHandle, HandleError> {
        match self {
            SimHandle::Hierarchy(h) => Ok(h),
            other => Err(other.wrong_kind("module")),
        }
    }

    pub fn as_signal(self) -> Result<LogicHandle, HandleError> {
        match self {
            SimHandle::Logic(l) => Ok(l),
            other => Err(other.wrong_kind("signal")),
        }
    }

    pub fn as_real(self) -> Result<RealHandle, HandleError> {
        match self {
            Self::Real(h) => Ok(h),
            other => Err(other.wrong_kind("real")),
        }
    }

    pub fn as_string(self) -> Result<StringHandle, HandleError> {
        match self {
            Self::String(h) => Ok(h),
            other => Err(other.wrong_kind("string")),
        }
    }

    pub fn as_aggregate(self) -> Result<AggregateHandle, HandleError> {
        match self {
            Self::Struct(h) | Self::Union(h) => Ok(h),
            other => Err(other.wrong_kind("struct or union")),
        }
    }

    pub fn is_other(self) -> bool {
        matches!(self, SimHandle::Other)
    }
}

/// A trait for the common methods of all handle types
pub trait HandleBase {
    /// Get the name of the handle (without hierarchy)
    fn name(&self) -> String;

    /// Get the full name of the handle (with hierarchy)
    fn full_name(&self) -> String;
}

/// A trait for handles that have children (hierarchy, aggregate)
pub trait HandleChildren {
    /// Get a child handle by name
    fn child(&self, name: &str) -> Result<SimHandle, HandleError>;

    /// Get all child handles
    fn children(&self) -> Vec<SimHandle>;

    /// Get a child handle by name and convert it to logic handle
    fn signal(&self, name: &str) -> Result<LogicHandle, HandleError> {
        self.child(name)?.as_signal()
    }

    /// Get a child handle by name and convert it to real handle
    fn real(&self, name: &str) -> Result<RealHandle, HandleError> {
        self.child(name)?.as_real()
    }

    /// Get a child handle by name and convert it to string handle
    fn string(&self, name: &str) -> Result<StringHandle, HandleError> {
        self.child(name)?.as_string()
    }

    /// Get a child handle by name and convert it to hierarchy handle
    fn hierarchy(&self, name: &str) -> Result<HierarchyHandle, HandleError> {
        self.child(name)?.as_hierarchy()
    }

    /// Get a child handle by name and convert it to aggregate handle
    fn aggregate(&self, name: &str) -> Result<AggregateHandle, HandleError> {
        self.child(name)?.as_aggregate()
    }
}

/// A trait for handles that have observable events
pub trait HandleEvent {
    /// Get the rising edge event for this handle
    fn rising_edge(&self) -> Edge;

    /// Get the falling edge event for this handle
    fn falling_edge(&self) -> Edge;

    /// Get the value change event for this handle
    fn value_change(&self) -> Edge;
}
