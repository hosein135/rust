//! Analysis: one write, every subscriber hears it (D86–D88; pyuvm
//! `uvm_analysis_port`, `uvm_subscriber`, `uvm_tlm_analysis_fifo`).
//!
//! # Analysis is not a queue
//!
//! [`AnalysisBus`] and [`TlmFifo`](crate::TlmFifo) share three letters and
//! nothing else. A `TlmFifo` is a *queue*: one consumer takes each item, the
//! producer blocks when it is full, and the item is gone once taken. An
//! `AnalysisBus` is a *broadcast*, and **it has no queue at all**: `write`
//! calls every subscriber and returns. Nothing is stored, so a write with no
//! subscribers is not buffered for later — it is simply gone, which is legal
//! and is what a monitor nobody listens to should cost. Do not reach for one
//! expecting the other.
//!
//! To keep the traffic, *subscribe and keep it*: a subscriber's `write` puts
//! the item wherever that component wants it — a `Vec`, an unbounded
//! `TlmFifo`, a comparison against a prediction. The hub is not the memory;
//! the subscriber is.
//!
//! # Why delivery is synchronous
//!
//! A monitor writes a transaction and moves on within the same simulation
//! instant — the time wheel must not turn because a scoreboard was listening.
//! So `write` is not `async`: the publisher's call runs every subscriber's
//! handler and returns.
//!
//! That is only possible because a subscriber shares its **state** rather than
//! itself. A handler needs `&mut` its data, and no component can hand out
//! `&mut self` to a sibling — so the data lives in a
//! [`RustdvShared`](crate::RustdvShared), the component keeps one handle, and
//! the port gets another. An earlier design queued items and delivered them
//! later; "later" is exactly what analysis must not do.
//!
//! # One connection idiom (Ray, 2026-07-24)
//!
//! The UVM broadcasts straight from a source's analysis port to subscribers.
//! rustdv's components are erased, so neither side can reach the other, and
//! analysis gets a **hub** for the same reason put/get has a FIFO: a concrete
//! `#[component]` child that the parent owns and can wire.
//!
//! ```ignore
//! self.bus.pub_export().connect(&self.mon, Monitor::AP);
//! self.bus.sub_export().connect(&self.sb, Scoreboard::INPUT);
//! self.bus.sub_export().connect(&self.cov, Coverage::INPUT);
//! ```
//!
//! Several subscribers on one `sub_export()` is what makes it a broadcast.
//! This is a deliberate divergence from IEEE 1800.2 — which we are not
//! implementing — and one idiom to learn beats two.

use std::cell::RefCell;
use std::rc::Rc;

use crate::component::{Component, ComponentNode};
use crate::port::{PortName, PortOwner, PublishIf, SinkHandle, bind_or_panic, sink_of};

// ===========================================================================
// The hub
// ===========================================================================

/// What every handle to one hub points at. A subscriber list, and nothing
/// else — there is no queue here by design (D90).
struct HubInner<T: 'static> {
    subs: RefCell<Vec<Rc<dyn SinkHandle<T>>>>,
}

impl<T: 'static> HubInner<T> {
    /// Hand the item to every subscriber, in connection order, and return.
    ///
    /// No queue, no clone of the item, no yield: subscribers see a `&T` and
    /// take from it what they want to keep.
    fn broadcast(&self, item: &T) {
        // Cloned out of the RefCell first: a subscriber's handler is free to
        // do anything, and this loop must not hold a borrow while it runs.
        let subs: Vec<Rc<dyn SinkHandle<T>>> = self.subs.borrow().clone();
        for sub in subs {
            sub.deliver(item);
        }
    }
}

impl<T: 'static> PublishIf<T> for HubInner<T> {
    fn write(&self, item: &T) {
        self.broadcast(item);
    }
}

/// The publish side of a hub: connect it to a source's
/// [`PublishPort`](crate::PublishPort).
pub struct PublishExport<T: 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: 'static> PublishExport<T> {
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn PublishIf<T>>) {
        bind_or_panic(owner, name, self.inner.clone() as Rc<dyn PublishIf<T>>);
    }
}

/// The subscribe side of a hub. Connect as many subscribers to it as you
/// like — that is what makes the write a broadcast.
pub struct SubscribeExport<T: 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: 'static> SubscribeExport<T> {
    /// Take the component's subscriber and add it to the broadcast list.
    ///
    /// Unlike a put/get connect, nothing is written *into* the port: the
    /// component already put its subscriber there with `subscribe`, and the
    /// hub collects it. Broadcast runs the other way, so the wiring does too.
    pub fn connect(&self, owner: &dyn PortOwner, name: PortName<dyn SinkHandle<T>>) {
        match sink_of(owner, name) {
            Ok(sink) => self.inner.subs.borrow_mut().push(sink),
            Err(e) => panic!("{e}"),
        }
    }
}

/// A broadcast hub: one publisher in, every subscriber out.
///
/// **It holds no items.** `write` calls each subscriber and returns; if nobody
/// is subscribed the datum is gone (D90). A component that needs to keep the
/// traffic subscribes and keeps it — in a `Vec`, or in an unbounded `TlmFifo`
/// it owns, if it wants to pull on its own schedule.
///
/// Declare it as a child with `#[component]`, like a `TlmFifo`, then hand
/// out its exports in `connect`: [`pub_export`](Self::pub_export) for the
/// source, [`sub_export`](Self::sub_export) for each listener.
pub struct AnalysisBus<T: 'static> {
    inner: Rc<HubInner<T>>,
}

impl<T: 'static> Clone for AnalysisBus<T> {
    /// Another handle to the *same* hub.
    fn clone(&self) -> Self {
        AnalysisBus {
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static> Default for AnalysisBus<T> {
    fn default() -> Self {
        AnalysisBus::new()
    }
}

impl<T: 'static> AnalysisBus<T> {
    pub fn new() -> AnalysisBus<T> {
        AnalysisBus {
            inner: Rc::new(HubInner {
                subs: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The publish side, for a source's `PublishPort`.
    pub fn pub_export(&self) -> PublishExport<T> {
        PublishExport {
            inner: self.inner.clone(),
        }
    }

    /// The subscribe side, for a subscriber's `SubscribePort`. Connect
    /// several; each one sees every item.
    pub fn sub_export(&self) -> SubscribeExport<T> {
        SubscribeExport {
            inner: self.inner.clone(),
        }
    }

    /// Broadcast an item, as the owner of the hub rather than through a port.
    pub fn write(&self, item: &T) {
        self.inner.broadcast(item);
    }

    /// How many subscribers are listening. Zero is legal.
    pub fn subscriber_count(&self) -> usize {
        self.inner.subs.borrow().len()
    }
}

// A hub is a component: it appears in the hierarchy and its phases are no-ops.
impl<T: 'static> Component for AnalysisBus<T> {}

impl<T: 'static> ComponentNode for AnalysisBus<T> {
    fn node_name(&self) -> &'static str {
        "AnalysisBus"
    }
    fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
        Vec::new()
    }
}

// ===========================================================================
// Tests — no simulator. The broadcast is synchronous by design (D87).
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{
        PortField, PortName, PortOwner, PublishPort, SinkHandle, SubscribePort, Subscriber,
    };
    use crate::shared::RustdvShared;
    use std::any::Any;

    #[derive(Default)]
    struct Tally {
        seen: Vec<u8>,
    }
    impl Subscriber<u8> for Tally {
        fn write(&mut self, item: &u8) {
            self.seen.push(*item);
        }
    }

    struct Source {
        ap: PublishPort<u8>,
    }
    impl Source {
        const AP: PortName<dyn PublishIf<u8>> = PortName::new("ap");
    }
    impl PortOwner for Source {
        fn owner_port_slot(&self, name: &str) -> Option<Rc<dyn Any>> {
            (name == "ap").then(|| self.ap.slot_any())
        }
        fn owner_label(&self) -> &'static str {
            "Source"
        }
    }

    struct Listener {
        input: SubscribePort<u8>,
        tally: RustdvShared<Tally>,
    }
    impl Listener {
        const INPUT: PortName<dyn SinkHandle<u8>> = PortName::new("input");
        fn new() -> Listener {
            let l = Listener {
                input: SubscribePort::default(),
                tally: RustdvShared::default(),
            };
            l.input.subscribe(l.tally.clone());
            l
        }
    }
    impl PortOwner for Listener {
        fn owner_port_slot(&self, name: &str) -> Option<Rc<dyn Any>> {
            (name == "input").then(|| self.input.slot_any())
        }
        fn owner_label(&self) -> &'static str {
            "Listener"
        }
    }

    #[test]
    fn one_write_reaches_every_subscriber() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let src = Source {
            ap: PublishPort::default(),
        };
        let a = Listener::new();
        let b = Listener::new();

        bus.pub_export().connect(&src, Source::AP);
        bus.sub_export().connect(&a, Listener::INPUT);
        bus.sub_export().connect(&b, Listener::INPUT);
        assert_eq!(bus.subscriber_count(), 2);

        src.ap.write(&7);
        assert_eq!(a.tally.get().seen, vec![7]);
        assert_eq!(
            b.tally.get().seen,
            vec![7],
            "several subscribers is what makes it a broadcast"
        );
    }

    #[test]
    fn subscribers_are_called_in_connection_order() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let src = Source {
            ap: PublishPort::default(),
        };
        let first = Listener::new();
        let second = Listener::new();
        bus.pub_export().connect(&src, Source::AP);
        bus.sub_export().connect(&first, Listener::INPUT);
        bus.sub_export().connect(&second, Listener::INPUT);

        for n in 1..=3u8 {
            src.ap.write(&n);
        }
        assert_eq!(first.tally.get().seen, vec![1, 2, 3]);
        assert_eq!(second.tally.get().seen, vec![1, 2, 3]);
    }

    /// D90: the hub holds nothing. A datum broadcast to nobody is gone, and a
    /// subscriber connected afterwards does not receive it.
    #[test]
    fn the_bus_stores_nothing() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let src = Source {
            ap: PublishPort::default(),
        };
        bus.pub_export().connect(&src, Source::AP);

        src.ap.write(&1); // nobody is listening
        src.ap.write(&2);

        let late = Listener::new();
        bus.sub_export().connect(&late, Listener::INPUT);
        assert!(
            late.tally.get().seen.is_empty(),
            "nothing was buffered for a late subscriber"
        );

        src.ap.write(&3);
        assert_eq!(
            late.tally.get().seen,
            vec![3],
            "only what arrives after it connects"
        );
    }

    /// D85: analysis has min cardinality 0 — a monitor nobody listens to is a
    /// legitimate testbench, and `write` on an unconnected port is legal.
    #[test]
    fn writing_with_no_subscribers_is_legal() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let src = Source {
            ap: PublishPort::default(),
        };
        bus.pub_export().connect(&src, Source::AP);
        assert_eq!(bus.subscriber_count(), 0);
        src.ap.write(&1); // must not panic
    }

    #[test]
    fn an_unconnected_publish_port_does_not_panic() {
        let src = Source {
            ap: PublishPort::default(),
        };
        assert!(!src.ap.has_subscribers());
        src.ap.write(&1); // a source nobody wired is still a valid testbench
    }

    /// D87: delivery is synchronous — the handler has already run by the time
    /// `write` returns, with no `await` anywhere in the path.
    #[test]
    fn delivery_happens_before_write_returns() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let src = Source {
            ap: PublishPort::default(),
        };
        let sub = Listener::new();
        bus.pub_export().connect(&src, Source::AP);
        bus.sub_export().connect(&sub, Listener::INPUT);

        src.ap.write(&5);
        assert_eq!(
            sub.tally.get().seen,
            vec![5],
            "already delivered, no scheduling in between"
        );
    }

    /// A component that never called `subscribe` has nothing to receive with,
    /// and connecting it says so by name rather than dropping items silently.
    #[test]
    #[should_panic(expected = "has no subscriber")]
    fn connecting_a_subscriber_with_no_sink_is_a_named_error() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let bare = Listener {
            input: SubscribePort::default(),
            tally: RustdvShared::default(),
        };
        bus.sub_export().connect(&bare, Listener::INPUT);
    }

    #[test]
    fn a_clone_is_the_same_bus() {
        let bus: AnalysisBus<u8> = AnalysisBus::new();
        let other = bus.clone();
        let sub = Listener::new();
        other.sub_export().connect(&sub, Listener::INPUT);
        assert_eq!(
            bus.subscriber_count(),
            1,
            "two handles, one subscriber list"
        );
        bus.write(&4);
        assert_eq!(sub.tally.get().seen, vec![4]);
    }
}
