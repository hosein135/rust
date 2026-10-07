//! Sim-aware synchronization primitives: `Event` and FIFO-fair `Lock`
//! (design-doc mapping row 17 — std/tokio primitives don't know sim time).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

// ---------------------------------------------------------------------------
// Event: manual-reset event (cocotb: _base_triggers.py, _Event)
// ---------------------------------------------------------------------------

struct EventInner {
    is_set: Cell<bool>,
    waiters: RefCell<Vec<Waker>>,
}

/// Manual-reset event; `wait()` completes immediately if already set.
#[derive(Clone)]
pub struct Event {
    inner: Rc<EventInner>,
}

impl Event {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Event {
        Event {
            inner: Rc::new(EventInner {
                is_set: Cell::new(false),
                waiters: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn set(&self) {
        self.inner.is_set.set(true);
        for w in self.inner.waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }

    pub fn clear(&self) {
        self.inner.is_set.set(false);
    }

    pub fn is_set(&self) -> bool {
        self.inner.is_set.get()
    }

    pub fn wait(&self) -> EventWait {
        EventWait {
            inner: self.inner.clone(),
        }
    }
}

pub struct EventWait {
    inner: Rc<EventInner>,
}

impl Future for EventWait {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.inner.is_set.get() {
            Poll::Ready(())
        } else {
            self.inner.waiters.borrow_mut().push(cx.waker().clone());
            Poll::Pending
        }
    }
}

// ---------------------------------------------------------------------------
// Lock: FIFO-fair mutex (cocotb Lock docstring: "Guarantees fair scheduling")
// ---------------------------------------------------------------------------

struct LockWaiter {
    granted: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

struct LockInner {
    locked: Cell<bool>,
    waiters: RefCell<VecDeque<Rc<LockWaiter>>>,
}

impl LockInner {
    /// Pass ownership to the next waiter, or unlock.
    fn release(&self) {
        match self.waiters.borrow_mut().pop_front() {
            Some(w) => {
                w.granted.set(true);
                if let Some(waker) = w.waker.borrow_mut().take() {
                    waker.wake();
                }
                // Ownership transferred (still locked).
            }
            None => {
                self.locked.set(false);
            }
        }
    }
}

#[derive(Clone)]
pub struct Lock {
    inner: Rc<LockInner>,
}

impl Lock {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Lock {
        Lock {
            inner: Rc::new(LockInner {
                locked: Cell::new(false),
                waiters: RefCell::new(VecDeque::new()),
            }),
        }
    }

    pub fn locked(&self) -> bool {
        self.inner.locked.get()
    }

    pub fn acquire(&self) -> Acquire {
        Acquire {
            inner: self.inner.clone(),
            waiter: None,
            acquired: false,
        }
    }
}

pub struct Acquire {
    inner: Rc<LockInner>,
    waiter: Option<Rc<LockWaiter>>,
    acquired: bool,
}

impl Future for Acquire {
    type Output = LockGuard;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<LockGuard> {
        match &self.waiter {
            None => {
                if !self.inner.locked.get() {
                    self.inner.locked.set(true);
                    self.acquired = true;
                    Poll::Ready(LockGuard {
                        inner: self.inner.clone(),
                    })
                } else {
                    let w = Rc::new(LockWaiter {
                        granted: Cell::new(false),
                        waker: RefCell::new(Some(cx.waker().clone())),
                    });
                    self.inner.waiters.borrow_mut().push_back(w.clone());
                    self.waiter = Some(w);
                    Poll::Pending
                }
            }
            Some(w) => {
                if w.granted.get() {
                    self.acquired = true;
                    Poll::Ready(LockGuard {
                        inner: self.inner.clone(),
                    })
                } else {
                    *w.waker.borrow_mut() = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

impl Drop for Acquire {
    fn drop(&mut self) {
        if self.acquired {
            return; // guard owns the lock now
        }
        if let Some(w) = &self.waiter {
            if w.granted.get() {
                // Granted but never polled (task cancelled): pass it on.
                self.inner.release();
            } else {
                // Remove ourselves from the queue.
                self.inner
                    .waiters
                    .borrow_mut()
                    .retain(|x| !Rc::ptr_eq(x, w));
            }
        }
    }
}

/// RAII lock guard; drop releases (fair handoff to the next waiter).
pub struct LockGuard {
    inner: Rc<LockInner>,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        self.inner.release();
    }
}

// ===========================================================================
// Tests — no simulator.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor;
    use crate::testing::{assert_pending, block_on};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn wait_before_set_wakes() {
        block_on(async {
            let ev = Event::new();
            let setter = ev.clone();
            executor::spawn(async move {
                setter.set();
            });
            ev.wait().await; // returns only because the spawned task set it
        });
    }

    /// Events are **level**-triggered with an explicit `clear`, not edge:
    /// `is_set` is sticky, so a `set` that happens before anyone waits is
    /// still there when they arrive. This is what makes the sequencer
    /// handshake safe — a driver may grant an item before the sequence gets
    /// round to waiting for the grant, and the sequence still proceeds.
    #[test]
    fn set_latches_until_cleared() {
        block_on(async {
            let ev = Event::new();
            ev.set();
            ev.wait().await; // returns at once: the set is remembered
            assert!(ev.is_set());
        });
    }

    #[test]
    fn clear_makes_a_later_wait_block_again() {
        let ev = Event::new();
        ev.set();
        ev.clear();
        assert!(!ev.is_set());
        assert_pending(async move { ev.wait().await });
    }

    #[test]
    fn one_set_wakes_every_waiter() {
        block_on(async {
            let ev = Event::new();
            let woken = Rc::new(RefCell::new(0));
            for _ in 0..3 {
                let (e, w) = (ev.clone(), woken.clone());
                executor::spawn(async move {
                    e.wait().await;
                    *w.borrow_mut() += 1;
                });
            }
            // Let the three waiters register before firing.
            crate::executor::current().run_until_idle();
            ev.set();
            crate::executor::current().run_until_idle();
            assert_eq!(*woken.borrow(), 3, "all three saw one set");
        });
    }

    #[test]
    fn lock_is_fifo_fair() {
        block_on(async {
            let lock = Lock::new();
            let order = Rc::new(RefCell::new(Vec::new()));

            let held = lock.acquire().await; // the test holds it first

            for who in 1..=3 {
                let (l, o) = (lock.clone(), order.clone());
                executor::spawn(async move {
                    let _g = l.acquire().await;
                    o.borrow_mut().push(who);
                });
                // Each spawn runs far enough to queue for the lock, in order.
                crate::executor::current().run_until_idle();
            }

            drop(held);
            crate::executor::current().run_until_idle();
            assert_eq!(*order.borrow(), vec![1, 2, 3], "granted in request order");
        });
    }
}
