//! Sim-aware FIFO queue (port of `cocotb.queue.Queue`, mapping row 18 —
//! the foundation for TLM FIFOs, as in pyuvm).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

struct QInner<T> {
    buf: RefCell<VecDeque<T>>,
    cap: Option<usize>, // None = unbounded
    get_waiters: RefCell<Vec<Waker>>,
    put_waiters: RefCell<Vec<Waker>>,
}

impl<T> QInner<T> {
    fn has_space(&self) -> bool {
        match self.cap {
            None => true,
            Some(c) => self.buf.borrow().len() < c,
        }
    }
    fn wake_getters(&self) {
        for w in self.get_waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }
    fn wake_putters(&self) {
        for w in self.put_waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }
}

/// FIFO queue with optional bound; cloning shares the queue.
pub struct Queue<T> {
    inner: Rc<QInner<T>>,
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Queue {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Queue<T> {
    /// `cap = None` → unbounded; `Some(n)` → bounded at n.
    pub fn new(cap: Option<usize>) -> Queue<T> {
        Queue {
            inner: Rc::new(QInner {
                buf: RefCell::new(VecDeque::new()),
                cap,
                get_waiters: RefCell::new(Vec::new()),
                put_waiters: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn unbounded() -> Queue<T> {
        Self::new(None)
    }

    pub fn len(&self) -> usize {
        self.inner.buf.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.buf.borrow().is_empty()
    }

    /// Is there room right now? (Zero time; the answer can go stale as soon
    /// as another task runs.)
    pub fn has_space(&self) -> bool {
        self.inner.has_space()
    }

    pub fn try_put(&self, item: T) -> Result<(), T> {
        if self.inner.has_space() {
            self.inner.buf.borrow_mut().push_back(item);
            self.inner.wake_getters();
            Ok(())
        } else {
            Err(item)
        }
    }

    pub fn try_get(&self) -> Option<T> {
        let item = self.inner.buf.borrow_mut().pop_front();
        if item.is_some() {
            self.inner.wake_putters();
        }
        item
    }

    pub fn put(&self, item: T) -> Put<T> {
        Put {
            inner: self.inner.clone(),
            item: Some(item),
        }
    }

    pub fn get(&self) -> Get<T> {
        Get {
            inner: self.inner.clone(),
        }
    }

    /// Wait until the queue has room, **without** handing over an item.
    ///
    /// For callers that must do something with the item at the instant it is
    /// accepted — a TLM FIFO's analysis tap, which broadcasts each item as it
    /// goes in. `put` takes ownership, so by the time it returns there is
    /// nothing left to show anyone; this splits the wait from the handover:
    ///
    /// ```ignore
    /// q.wait_for_space().await;
    /// tap(&item);            // no await between these two lines, so no
    /// let _ = q.try_put(item); // other task can take the space first
    /// ```
    pub fn wait_for_space(&self) -> Space<T> {
        Space {
            inner: self.inner.clone(),
        }
    }
}

/// Resolves when the queue has room. See [`Queue::wait_for_space`].
pub struct Space<T> {
    inner: Rc<QInner<T>>,
}

impl<T> Future for Space<T> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.inner.has_space() {
            Poll::Ready(())
        } else {
            self.inner.put_waiters.borrow_mut().push(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T: Clone> Queue<T> {
    /// The front item **without removing it** (TLM `peek`).
    ///
    /// A copy, so the item stays in the queue for whoever gets it next. That
    /// is why `peek` needs `T: Clone` and `get` does not: `get` hands over
    /// ownership, `peek` cannot.
    pub fn try_peek(&self) -> Option<T> {
        self.inner.buf.borrow().front().cloned()
    }

    /// Block until there is something to peek at, then copy it.
    pub fn peek(&self) -> Peek<T> {
        Peek {
            inner: self.inner.clone(),
        }
    }
}

pub struct Peek<T> {
    inner: Rc<QInner<T>>,
}

impl<T: Clone> Future for Peek<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let item = self.inner.buf.borrow().front().cloned();
        match item {
            // No `wake_putters`: nothing left the queue, so no space opened up.
            Some(v) => Poll::Ready(v),
            None => {
                self.inner.get_waiters.borrow_mut().push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub struct Put<T> {
    inner: Rc<QInner<T>>,
    item: Option<T>,
}

impl<T> Future for Put<T> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // T may be !Unpin, but we never project into it — safe to get_mut
        // via unsafe-free approach: Put is structurally Unpin because we
        // only move `item` out of an Option.
        let this = unsafe { self.get_unchecked_mut() };
        if this.inner.has_space() {
            let item = this.item.take().expect("Put polled after completion");
            this.inner.buf.borrow_mut().push_back(item);
            this.inner.wake_getters();
            Poll::Ready(())
        } else {
            this.inner.put_waiters.borrow_mut().push(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct Get<T> {
    inner: Rc<QInner<T>>,
}

impl<T> Future for Get<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let item = self.inner.buf.borrow_mut().pop_front();
        match item {
            Some(v) => {
                self.inner.wake_putters();
                Poll::Ready(v)
            }
            None => {
                self.inner.get_waiters.borrow_mut().push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

// ===========================================================================
// Tests — no simulator: a Queue is pure async, with no `gpi::` anywhere.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_pending, block_on};

    #[test]
    fn put_get_round_trips_in_order() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            for n in 0..5 {
                q.put(n).await;
            }
            for n in 0..5 {
                assert_eq!(q.get().await, n, "a queue is FIFO");
            }
        });
    }

    /// D89: a refused `try_put` hands the item **back**. A bare "no" would
    /// swallow a transaction that was never delivered.
    #[test]
    fn try_put_on_a_full_queue_returns_the_item() {
        block_on(async {
            let q: Queue<String> = Queue::new(Some(1));
            assert!(q.try_put(String::from("first")).is_ok());
            match q.try_put(String::from("second")) {
                Err(back) => assert_eq!(back, "second", "the item comes home"),
                Ok(()) => panic!("a depth-1 queue accepted a second item"),
            }
        });
    }

    #[test]
    fn try_get_on_an_empty_queue_is_none() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            assert!(q.try_get().is_none());
            q.put(1).await;
            assert_eq!(q.try_get(), Some(1));
            assert!(q.try_get().is_none(), "and it was consumed");
        });
    }

    #[test]
    fn a_blocked_get_wakes_on_a_put() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            let producer = q.clone();
            crate::executor::spawn(async move {
                producer.put(42).await;
            });
            assert_eq!(q.get().await, 42, "the get was waiting before the put");
        });
    }

    #[test]
    fn a_blocked_put_wakes_when_space_appears() {
        block_on(async {
            let q: Queue<u8> = Queue::new(Some(1));
            q.put(1).await;
            let consumer = q.clone();
            crate::executor::spawn(async move {
                let _ = consumer.get().await;
            });
            q.put(2).await; // blocks until the spawned get drains the first
            assert_eq!(q.len(), 1);
        });
    }

    #[test]
    fn a_get_on_an_empty_queue_waits() {
        let q: Queue<u8> = Queue::unbounded();
        assert_pending(async move { q.get().await });
    }

    /// Peek copies and leaves the item, which is why it needs `T: Clone`
    /// where `get` does not.
    #[test]
    fn peek_does_not_consume() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            q.put(9).await;
            assert_eq!(q.peek().await, 9);
            assert_eq!(q.len(), 1, "peek left it there");
            assert_eq!(q.get().await, 9, "and get takes the same item");
        });
    }

    #[test]
    fn try_peek_is_none_when_empty() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            assert!(q.try_peek().is_none());
            q.put(3).await;
            assert_eq!(q.try_peek(), Some(3));
            assert_eq!(q.len(), 1);
        });
    }

    #[test]
    fn has_space_agrees_with_len() {
        block_on(async {
            let q: Queue<u8> = Queue::new(Some(2));
            assert!(q.has_space());
            q.put(1).await;
            assert!(q.has_space());
            q.put(2).await;
            assert!(!q.has_space(), "full at its declared depth");
            assert_eq!(q.len(), 2);
        });
    }

    #[test]
    fn wait_for_space_returns_when_drained() {
        block_on(async {
            let q: Queue<u8> = Queue::new(Some(1));
            q.put(1).await;
            let consumer = q.clone();
            crate::executor::spawn(async move {
                let _ = consumer.get().await;
            });
            q.wait_for_space().await;
            assert!(q.has_space());
        });
    }

    #[test]
    fn unbounded_never_blocks_a_put() {
        block_on(async {
            let q: Queue<u32> = Queue::unbounded();
            for n in 0..1000 {
                assert!(q.try_put(n).is_ok(), "an unbounded queue always has room");
            }
            assert_eq!(q.len(), 1000);
        });
    }

    #[test]
    fn is_empty_tracks_contents() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            assert!(q.is_empty());
            q.put(1).await;
            assert!(!q.is_empty());
            let _ = q.get().await;
            assert!(q.is_empty());
        });
    }

    /// A dropped `get` future must not have consumed an item — otherwise
    /// losing a race (D82c) would silently eat a transaction.
    #[test]
    fn a_dropped_get_consumes_nothing() {
        block_on(async {
            let q: Queue<u8> = Queue::unbounded();
            {
                let pending = q.get();
                drop(pending);
            }
            q.put(5).await;
            assert_eq!(q.get().await, 5, "the item survived the dropped get");
        });
    }
}
