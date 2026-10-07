//! `RustdvShared<T>`: state two components can both see (D88).
//!
//! Analysis delivery has to be synchronous — the publisher calls `write` and
//! every subscriber's handler runs before control comes back, with no
//! `await` and no simulation time passing. But a subscriber's handler needs
//! `&mut subscriber` while the publisher's `run` holds `&mut publisher`, and
//! siblings in the tree cannot reach each other.
//!
//! The way out is to share the **state**, not the component. A subscriber
//! keeps its tally in a `RustdvShared<T>`, hands a second handle to its port,
//! and both see the same data. Delivery then mutates the tally without ever
//! touching the component.
//!
//! The name is deliberate. This is not a general Rust facility — someone who
//! goes looking for `Shared<T>` in the standard library will not find it — so
//! it wears the framework's name and says where it comes from.

use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::rc::Rc;

/// A handle to state shared between a component and its analysis port.
///
/// Cloning gives another handle to the *same* state, the way `Rc` does — it
/// does not copy the data.
pub struct RustdvShared<T> {
    inner: Rc<RefCell<T>>,
}

impl<T> RustdvShared<T> {
    pub fn new(value: T) -> RustdvShared<T> {
        RustdvShared {
            inner: Rc::new(RefCell::new(value)),
        }
    }

    /// Read the shared state.
    ///
    /// The guard borrows at run time, so holding one across a call that also
    /// reads is fine and holding one across a call that *writes* panics. Keep
    /// the guard short — usually one line, as in
    /// `let seen = self.tally.get();`.
    pub fn get(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    /// Modify the shared state.
    pub fn get_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    /// How many handles point at this state — the component's, the port's,
    /// and any others.
    pub fn handle_count(&self) -> usize {
        Rc::strong_count(&self.inner)
    }
}

impl<T> Clone for RustdvShared<T> {
    fn clone(&self) -> Self {
        RustdvShared {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Default> Default for RustdvShared<T> {
    fn default() -> Self {
        RustdvShared::new(T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for RustdvShared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RustdvShared({:?})", self.inner.borrow())
    }
}
