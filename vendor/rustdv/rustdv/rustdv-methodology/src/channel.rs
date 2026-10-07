//! Channels: the TLM-1 replacement (design-doc §5.6, review-memo R6).
//! Twelve pyuvm port classes become six methods on two types; direction
//! lives in the type name; a mismatch is a compile error.
//!
//! > **Face D3 before trusting this collapse.** "Twelve classes become six
//! > methods" is the same *make-it-static* reasoning that deleted
//! > build/connect. R6 itself stands — channels are a legitimate TLM-1
//! > port — but the TLM restoration (D17–D24) deliberately keeps the full
//! > UVM surface: ports live in components, exports on FIFOs, every UVM
//! > port name shipped (D19), and a `TlmFifo` that *encapsulates* the queue
//! > so two components connect through it without knowing each other. A
//! > future session extending TLM must not treat this six-method core as
//! > the whole story or re-collapse the port zoo away. See
//! > `output/.design-decisions.md`.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlmError {
    /// All endpoints on the other side have been dropped.
    Disconnected,
}

impl fmt::Display for TlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel disconnected")
    }
}
impl std::error::Error for TlmError {}

/// `try_send` failure: returns the item (pyuvm try_put returning False).
#[derive(Debug)]
pub struct TlmFull<T>(pub T);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlmEmpty;

struct ChanInner<T> {
    buf: RefCell<VecDeque<T>>,
    cap: usize,
    senders: Cell<usize>,
    receivers: Cell<usize>,
    send_waiters: RefCell<Vec<Waker>>,
    recv_waiters: RefCell<Vec<Waker>>,
}

impl<T> ChanInner<T> {
    fn has_space(&self) -> bool {
        self.buf.borrow().len() < self.cap
    }
    fn wake_senders(&self) {
        for w in self.send_waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }
    fn wake_receivers(&self) {
        for w in self.recv_waiters.borrow_mut().drain(..) {
            w.wake();
        }
    }
}

/// Create a bounded channel (design-doc §5.6 signature). The UVM TLM FIFO
/// default depth is 1; `capacity = 0` is coerced to 1.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Rc::new(ChanInner {
        buf: RefCell::new(VecDeque::new()),
        cap: capacity.max(1),
        senders: Cell::new(1),
        receivers: Cell::new(1),
        send_waiters: RefCell::new(Vec::new()),
        recv_waiters: RefCell::new(Vec::new()),
    });
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner },
    )
}

/// The put family (pyuvm: _s12, 12.2.5).
pub struct Sender<T> {
    inner: Rc<ChanInner<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.senders.set(self.inner.senders.get() + 1);
        Sender {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let n = self.inner.senders.get() - 1;
        self.inner.senders.set(n);
        if n == 0 {
            self.inner.wake_receivers();
        }
    }
}

impl<T> Sender<T> {
    /// Blocking put.
    pub fn send(&self, item: T) -> Send_<T> {
        Send_ {
            inner: self.inner.clone(),
            item: Some(item),
        }
    }

    /// Nonblocking put; returns the item on full.
    pub fn try_send(&self, item: T) -> Result<(), TlmFull<T>> {
        if self.inner.receivers.get() == 0 {
            return Err(TlmFull(item)); // nowhere for it to go
        }
        if self.inner.has_space() {
            self.inner.buf.borrow_mut().push_back(item);
            self.inner.wake_receivers();
            Ok(())
        } else {
            Err(TlmFull(item))
        }
    }

    pub fn can_send(&self) -> bool {
        self.inner.has_space() && self.inner.receivers.get() > 0
    }
}

pub struct Send_<T> {
    inner: Rc<ChanInner<T>>,
    item: Option<T>,
}

impl<T> Future for Send_<T> {
    type Output = Result<(), TlmError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.inner.receivers.get() == 0 {
            return Poll::Ready(Err(TlmError::Disconnected));
        }
        if this.inner.has_space() {
            let item = this.item.take().expect("Send polled after completion");
            this.inner.buf.borrow_mut().push_back(item);
            this.inner.wake_receivers();
            Poll::Ready(Ok(()))
        } else {
            this.inner
                .send_waiters
                .borrow_mut()
                .push(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// The get/peek families (pyuvm: _s12, 12.2.5).
pub struct Receiver<T> {
    inner: Rc<ChanInner<T>>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        self.inner.receivers.set(self.inner.receivers.get() + 1);
        Receiver {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let n = self.inner.receivers.get() - 1;
        self.inner.receivers.set(n);
        if n == 0 {
            self.inner.wake_senders();
        }
    }
}

impl<T> Receiver<T> {
    /// Blocking get.
    pub fn recv(&self) -> Recv<T> {
        Recv {
            inner: self.inner.clone(),
        }
    }

    pub fn try_recv(&self) -> Result<T, TlmEmpty> {
        match self.inner.buf.borrow_mut().pop_front() {
            Some(v) => {
                self.inner.wake_senders();
                Ok(v)
            }
            None => Err(TlmEmpty),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.buf.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.buf.borrow().is_empty()
    }
}

impl<T: Clone> Receiver<T> {
    /// Blocking peek: waits for an item, returns a copy without removing.
    pub fn peek(&self) -> Peek<T> {
        Peek {
            inner: self.inner.clone(),
        }
    }

    pub fn try_peek(&self) -> Result<T, TlmEmpty> {
        self.inner.buf.borrow().front().cloned().ok_or(TlmEmpty)
    }
}

pub struct Recv<T> {
    inner: Rc<ChanInner<T>>,
}

impl<T> Future for Recv<T> {
    type Output = Result<T, TlmError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let item = self.inner.buf.borrow_mut().pop_front();
        match item {
            Some(v) => {
                self.inner.wake_senders();
                Poll::Ready(Ok(v))
            }
            None => {
                if self.inner.senders.get() == 0 {
                    return Poll::Ready(Err(TlmError::Disconnected));
                }
                self.inner
                    .recv_waiters
                    .borrow_mut()
                    .push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub struct Peek<T: Clone> {
    inner: Rc<ChanInner<T>>,
}

impl<T: Clone> Future for Peek<T> {
    type Output = Result<T, TlmError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let front = self.inner.buf.borrow().front().cloned();
        match front {
            Some(v) => Poll::Ready(Ok(v)),
            None => {
                if self.inner.senders.get() == 0 {
                    return Poll::Ready(Err(TlmError::Disconnected));
                }
                self.inner
                    .recv_waiters
                    .borrow_mut()
                    .push(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
