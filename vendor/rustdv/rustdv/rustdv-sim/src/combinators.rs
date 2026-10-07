//! Ports of `First`/`Combine` (design-doc mapping row 16): future
//! combinators with drop-based cancellation of the losers — the cleanup
//! cocotb does manually with kill-on-completion tasks falls out of RAII.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::time::SimDuration;
use crate::triggers::Timer;

pub enum Either<A, B> {
    First(A),
    Second(B),
}

/// First of two futures; the loser is dropped (unsubscribing its trigger).
///
/// The futures are boxed behind lifetime `'a` rather than `'static` (D82), so
/// a future that *borrows* — a component's `run` borrowing the tree, a
/// sub-sequence borrowing its parent sequence — can be composed here. `'static`
/// futures satisfy any `'a`, so every earlier caller is unaffected.
pub struct First2<'a, A, B> {
    a: Pin<Box<dyn Future<Output = A> + 'a>>,
    b: Pin<Box<dyn Future<Output = B> + 'a>>,
}

pub fn first2<'a, FA, FB>(a: FA, b: FB) -> First2<'a, FA::Output, FB::Output>
where
    FA: Future + 'a,
    FB: Future + 'a,
{
    First2 {
        a: Box::pin(a),
        b: Box::pin(b),
    }
}

impl<A, B> Future for First2<'_, A, B> {
    type Output = Either<A, B>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = self.a.as_mut().poll(cx) {
            return Poll::Ready(Either::First(v));
        }
        if let Poll::Ready(v) = self.b.as_mut().poll(cx) {
            return Poll::Ready(Either::Second(v));
        }
        Poll::Pending
    }
}

/// Join of two futures (port of `Combine`; SystemVerilog's `fork...join`).
pub struct Join2<'a, A, B> {
    a: Pin<Box<dyn Future<Output = A> + 'a>>,
    b: Pin<Box<dyn Future<Output = B> + 'a>>,
    ra: Option<A>,
    rb: Option<B>,
}

pub fn join2<'a, FA, FB>(a: FA, b: FB) -> Join2<'a, FA::Output, FB::Output>
where
    FA: Future + 'a,
    FB: Future + 'a,
{
    Join2 {
        a: Box::pin(a),
        b: Box::pin(b),
        ra: None,
        rb: None,
    }
}

impl<A, B> Future for Join2<'_, A, B> {
    type Output = (A, B);
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<(A, B)> {
        // Sound: the inner futures are boxed (their pinning is their own),
        // and ra/rb are plain values we intentionally move on completion.
        let this = unsafe { self.get_unchecked_mut() };
        if this.ra.is_none()
            && let Poll::Ready(v) = this.a.as_mut().poll(cx)
        {
            this.ra = Some(v);
        }
        if this.rb.is_none()
            && let Poll::Ready(v) = this.b.as_mut().poll(cx)
        {
            this.rb = Some(v);
        }
        if this.ra.is_some() && this.rb.is_some() {
            Poll::Ready((this.ra.take().unwrap(), this.rb.take().unwrap()))
        } else {
            Poll::Pending
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "operation timed out")
    }
}
impl std::error::Error for TimeoutError {}

/// Run `fut` with a simulation-time timeout.
pub async fn with_timeout<'a, F>(fut: F, d: SimDuration) -> Result<F::Output, TimeoutError>
where
    F: Future + 'a,
{
    match first2(fut, Timer::new(d)).await {
        Either::First(v) => Ok(v),
        Either::Second(()) => Err(TimeoutError),
    }
}

/// Join *N* futures, where N is known only at run time (D82).
///
/// `join2` covers the fixed-arity case the `join!` macro expands to; this
/// covers a `Vec` built at run time — a parent joining however many children
/// it has, or a virtual sequence joining a list of sub-sequences. Every future
/// is polled on each wake until all have completed; results come back in the
/// original order. Like `Join2`, the futures may borrow (`'a`).
pub struct JoinAll<'a, T> {
    futs: Vec<Option<Pin<Box<dyn Future<Output = T> + 'a>>>>,
    out: Vec<Option<T>>,
}

pub fn join_all<'a, T>(futs: Vec<Pin<Box<dyn Future<Output = T> + 'a>>>) -> JoinAll<'a, T> {
    let n = futs.len();
    JoinAll {
        futs: futs.into_iter().map(Some).collect(),
        out: (0..n).map(|_| None).collect(),
    }
}

impl<T> Future for JoinAll<'_, T> {
    type Output = Vec<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Vec<T>> {
        // Sound: the inner futures are boxed (they own their pinning), and
        // `out` holds plain values we move out once every future is done.
        let this = unsafe { self.get_unchecked_mut() };
        let mut all_done = true;
        for (i, slot) in this.futs.iter_mut().enumerate() {
            if let Some(f) = slot {
                match f.as_mut().poll(cx) {
                    Poll::Ready(v) => {
                        this.out[i] = Some(v);
                        *slot = None; // drop the finished future
                    }
                    Poll::Pending => all_done = false,
                }
            }
        }
        if all_done {
            Poll::Ready(this.out.iter_mut().map(|o| o.take().unwrap()).collect())
        } else {
            Poll::Pending
        }
    }
}

/// `first!(a, b, ...)` — first completed future wins; losers are dropped.
#[macro_export]
macro_rules! first {
    ($a:expr, $b:expr $(,)?) => {
        $crate::combinators::first2($a, $b)
    };
    ($a:expr, $b:expr, $($rest:expr),+ $(,)?) => {
        $crate::combinators::first2($a, $crate::first!($b, $($rest),+))
    };
}

/// `join!(a, b, ...)` — wait for all.
#[macro_export]
macro_rules! join {
    ($a:expr, $b:expr $(,)?) => {
        $crate::combinators::join2($a, $b)
    };
    ($a:expr, $b:expr, $($rest:expr),+ $(,)?) => {
        $crate::combinators::join2($a, $crate::join!($b, $($rest),+))
    };
}

// ===========================================================================
// Tests — no simulator.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::block_on;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn join2_yields_both() {
        block_on(async {
            let (a, b) = join2(async { 1u8 }, async { "two" }).await;
            assert_eq!(a, 1);
            assert_eq!(b, "two");
        });
    }

    #[test]
    fn join_all_preserves_input_order() {
        block_on(async {
            let futs: Vec<Pin<Box<dyn Future<Output = u8>>>> = vec![
                Box::pin(async { 1 }),
                Box::pin(async { 2 }),
                Box::pin(async { 3 }),
            ];
            assert_eq!(join_all(futs).await, vec![1, 2, 3]);
        });
    }

    #[test]
    fn first2_returns_the_winner() {
        block_on(async {
            let ev = crate::sync::Event::new();
            let waiter = ev.clone();
            crate::executor::spawn(async move {
                ev.set();
            });
            match first2(async { 7u8 }, async move { waiter.wait().await }).await {
                Either::First(v) => assert_eq!(v, 7),
                Either::Second(()) => panic!("the ready future should have won"),
            }
        });
    }

    /// D82c in miniature: **losing a race means being dropped.** Racing the
    /// whole run tree instead of each component dropped the tree mid-phase
    /// and a test passed with its scoreboard never running.
    #[test]
    fn first2_drops_the_loser() {
        struct Tattle(Rc<RefCell<bool>>);
        impl Drop for Tattle {
            fn drop(&mut self) {
                *self.0.borrow_mut() = true;
            }
        }

        let dropped = Rc::new(RefCell::new(false));
        let flag = dropped.clone();
        block_on(async move {
            let never = crate::sync::Event::new();
            // The tattle is moved *into* the future from outside, so it is
            // dropped when the future is dropped — whether or not the future
            // ever got polled.
            let tattle = Tattle(flag);
            let loser = async move {
                let _t = tattle;
                never.wait().await;
            };
            let _ = first2(async { 1u8 }, loser).await;
        });
        assert!(
            *dropped.borrow(),
            "the losing future was dropped, not left running"
        );
    }

    /// The whole point of D82: a joined future may **borrow**, so a
    /// sub-sequence can use its parent's state. If this stops compiling, the
    /// combinators have regained a `'static` bound.
    #[test]
    fn joined_futures_may_borrow() {
        block_on(async {
            let owned = [1u8, 2, 3];
            let borrow_a = async { owned.len() };
            let borrow_b = async { owned[0] as usize };
            let (a, b) = join2(borrow_a, borrow_b).await;
            assert_eq!((a, b), (3, 1));
        });
    }
}
