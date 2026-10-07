//! Objections: distributed end-of-test consensus with RAII guards
//! (design-doc §5.3; pyuvm: ObjectionHandler, uvm_component.objection()).
//! Forgetting to drop is impossible; diagnostics (description, raise site)
//! are captured in the guard.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustdv_sim::sync::Event;

struct ObjInner {
    count: Cell<usize>,
    drained: Event,
    raised_ever: Cell<bool>,
    active: RefCell<Vec<String>>,
}

#[derive(Clone)]
pub struct ObjectionRegistry {
    inner: Rc<ObjInner>,
}

impl ObjectionRegistry {
    #[allow(clippy::new_without_default)]
    pub fn new() -> ObjectionRegistry {
        ObjectionRegistry {
            inner: Rc::new(ObjInner {
                count: Cell::new(0),
                drained: Event::new(),
                raised_ever: Cell::new(false),
                active: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn raise(&self, description: &str) -> ObjectionGuard {
        let inner = self.inner.clone();
        inner.count.set(inner.count.get() + 1);
        inner.raised_ever.set(true);
        inner.drained.clear();
        inner.active.borrow_mut().push(description.to_string());
        ObjectionGuard {
            inner,
            description: description.to_string(),
        }
    }

    pub fn count(&self) -> usize {
        self.inner.count.get()
    }

    /// Was an objection ever raised? The runner asks before awaiting
    /// consensus, so a Part II test that never objects is not scolded by
    /// `wait_all_dropped`'s pyuvm warning (D46: both front doors, one path).
    pub fn ever_raised(&self) -> bool {
        self.inner.raised_ever.get()
    }

    /// Wait for the run phase to end by objection consensus (D82/D82b).
    ///
    /// Unlike [`ObjectionRegistry::wait_all_dropped`], this takes no shortcut: it waits on the
    /// `drained` event, which is set only when a raised objection count falls
    /// back to zero. That is exactly the semantics the phaser needs to *race*
    /// against the run tree:
    ///
    /// - objections raised and later dropped → the event fires and the phase
    ///   ends, cancelling responder loops that never return;
    /// - no objection ever raised → the event never fires, so the run tree
    ///   decides when the phase ends (D46's second front door).
    ///
    /// The runner cannot ask `ever_raised()` up front, because at that moment
    /// no run body has executed and nothing has been raised yet.
    pub async fn wait_drained_event(&self) {
        self.inner.drained.wait().await;
    }

    /// Objection report for timeout diagnostics (pyuvm ObjectionHandler).
    pub fn active(&self) -> Vec<String> {
        self.inner.active.borrow().clone()
    }

    pub async fn wait_all_dropped(&self) {
        if !self.inner.raised_ever.get() {
            // pyuvm's run_phase_complete warning path, ported as-is.
            rustdv_sim::log::warning(
                "all_objections_dropped awaited but no objection was ever raised",
            );
            return;
        }
        if self.inner.count.get() == 0 {
            return;
        }
        self.inner.drained.wait().await;
    }
}

/// RAII objection. Drop = drop_objection (mapping row 32).
pub struct ObjectionGuard {
    inner: Rc<ObjInner>,
    description: String,
}

impl Drop for ObjectionGuard {
    fn drop(&mut self) {
        let mut active = self.inner.active.borrow_mut();
        if let Some(pos) = active.iter().position(|d| *d == self.description) {
            active.remove(pos);
        }
        drop(active);
        let n = self.inner.count.get().saturating_sub(1);
        self.inner.count.set(n);
        if n == 0 {
            self.inner.drained.set();
        }
    }
}

// ===========================================================================
// Tests — no simulator.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rustdv_sim::testing::{assert_pending, block_on};

    #[test]
    fn a_guard_raises_and_dropping_it_drops() {
        let reg = ObjectionRegistry::new();
        assert_eq!(reg.count(), 0);
        {
            let _g = reg.raise("stimulus");
            assert_eq!(reg.count(), 1);
        }
        assert_eq!(reg.count(), 0, "the guard dropped it");
    }

    #[test]
    fn nested_objections_end_only_at_the_last_drop() {
        let reg = ObjectionRegistry::new();
        let a = reg.raise("a");
        let b = reg.raise("b");
        assert_eq!(reg.count(), 2);
        drop(a);
        assert_eq!(reg.count(), 1, "one left");
        drop(b);
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn drained_fires_when_a_raised_count_returns_to_zero() {
        block_on(async {
            let reg = ObjectionRegistry::new();
            let g = reg.raise("work");
            let waiter = reg.clone();
            rustdv_sim::executor::spawn(async move {
                waiter.wait_drained_event().await;
            });
            drop(g);
            // The waiter completes; if it did not, block_on would time out.
            reg.wait_drained_event().await;
        });
    }

    /// The D82b bug, as a regression test. Arming the race on "has anything
    /// ever objected?" answered `false` before any run body had executed, so
    /// the race was never armed and every responder-style testbench hung.
    /// The event's own semantics carry D46's rule instead: never objecting
    /// simply never fires.
    #[test]
    fn never_objecting_never_fires_the_drained_event() {
        let reg = ObjectionRegistry::new();
        assert_eq!(reg.count(), 0);
        assert_pending(async move { reg.wait_drained_event().await });
    }
}
