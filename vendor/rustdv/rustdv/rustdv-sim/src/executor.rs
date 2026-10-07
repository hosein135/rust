//! The bespoke single-threaded executor (design-doc §4.2/§4.3).
//!
//! Port of cocotb's event loop + Task model: a run queue drained to
//! exhaustion after every simulator callback (cocotb: `_event_loop.py`,
//! `EventLoop.run`), tasks with the same seven observable states
//! (cocotb: `task.py`, `_TaskState`), and drop-based cancellation (§4.6).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

pub type TaskId = u64;

/// The seven task states, per cocotb (mapping row 5).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskState {
    Unstarted,
    Scheduled,
    Running,
    Pending,
    Finished,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone)]
pub enum TaskError {
    Cancelled,
    Panicked(String),
    /// result() called before completion, or awaited twice.
    InvalidState,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Cancelled => write!(f, "task was cancelled"),
            TaskError::Panicked(m) => write!(f, "task panicked: {m}"),
            TaskError::InvalidState => write!(f, "task result not available"),
        }
    }
}
impl std::error::Error for TaskError {}

// ---------------------------------------------------------------------------
// Waker plumbing: the TriggerWaker role (design-doc §4.3). Waking pushes the
// task id onto a queue the executor drains; the Mutex is uncontended
// (single thread) and exists only to satisfy `Waker`'s Send+Sync contract.
// ---------------------------------------------------------------------------

struct WokenQueue(Mutex<VecDeque<TaskId>>);

struct TaskWaker {
    id: TaskId,
    woken: Arc<WokenQueue>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.woken.0.lock().unwrap().push_back(self.id);
    }
}

// ---------------------------------------------------------------------------
// Task bookkeeping
// ---------------------------------------------------------------------------

/// Per-task shared cell between the executor entry and its TaskHandle.
struct TaskShared<T> {
    state: Cell<TaskState>,
    result: RefCell<Option<Result<T, TaskError>>>,
    joiners: RefCell<Vec<Waker>>,
}

impl<T> TaskShared<T> {
    fn complete(&self, r: Result<T, TaskError>) {
        *self.result.borrow_mut() = Some(r);
        for w in self.joiners.borrow_mut().drain(..) {
            w.wake();
        }
    }
}

struct TaskEntry {
    fut: Pin<Box<dyn Future<Output = ()>>>,
    name: String,
    state: TaskState, // mirror for the executor's own bookkeeping
    /// Type-erased hook: propagate abnormal termination into TaskShared<T>.
    on_abort: Rc<dyn Fn(TaskError)>,
    /// Mirror of shared state for handle.state() queries.
    state_cell: Rc<dyn Fn(TaskState)>,
}

type FailureSink = Box<dyn Fn(&str)>;

struct ExecInner {
    tasks: RefCell<HashMap<TaskId, TaskEntry>>,
    next_id: Cell<TaskId>,
    run_queue: RefCell<VecDeque<TaskId>>,
    woken: Arc<WokenQueue>,
    running: Cell<bool>,
    currently_polling: Cell<Option<TaskId>>,
    /// Self-cancellations deferred until the poll returns.
    cancel_pending: RefCell<Vec<TaskId>>,
    /// Called whenever any task panics — the runner points this at
    /// "fail the current test" (cocotb: TestManager._task_done_callback).
    failure_sink: RefCell<Option<FailureSink>>,
}

/// The executor (design-doc §4.3). `!Send` — it never leaves the sim thread.
#[derive(Clone)]
pub struct Executor {
    inner: Rc<ExecInner>,
}

thread_local! {
    static CURRENT: RefCell<Option<Executor>> = const { RefCell::new(None) };
}

/// Create and install a fresh executor on this thread.
pub fn init() -> Executor {
    let ex = Executor {
        inner: Rc::new(ExecInner {
            tasks: RefCell::new(HashMap::new()),
            next_id: Cell::new(1),
            run_queue: RefCell::new(VecDeque::new()),
            woken: Arc::new(WokenQueue(Mutex::new(VecDeque::new()))),
            running: Cell::new(false),
            currently_polling: Cell::new(None),
            cancel_pending: RefCell::new(Vec::new()),
            failure_sink: RefCell::new(None),
        }),
    };
    CURRENT.with(|c| *c.borrow_mut() = Some(ex.clone()));
    ex
}

/// The thread's executor. Panics if `init()` has not run.
pub fn current() -> Executor {
    CURRENT.with(|c| c.borrow().clone().expect("rustdv executor not initialized"))
}

/// Port of `cocotb.start_soon` (mapping row 4).
pub fn spawn<F>(fut: F) -> TaskHandle<F::Output>
where
    F: Future + 'static,
{
    current().spawn_named(fut, None)
}

pub fn spawn_named<F>(fut: F, name: &str) -> TaskHandle<F::Output>
where
    F: Future + 'static,
{
    current().spawn_named(fut, Some(name))
}

impl Executor {
    pub fn spawn_named<F>(&self, fut: F, name: Option<&str>) -> TaskHandle<F::Output>
    where
        F: Future + 'static,
    {
        let id = self.inner.next_id.get();
        self.inner.next_id.set(id + 1);
        let name = name
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("task_{id}"));

        let shared = Rc::new(TaskShared::<F::Output> {
            state: Cell::new(TaskState::Unstarted),
            result: RefCell::new(None),
            joiners: RefCell::new(Vec::new()),
        });

        // Wrapper future: writes the value into the shared cell on success.
        let sh = shared.clone();
        let wrapped = async move {
            let out = fut.await;
            sh.state.set(TaskState::Finished);
            sh.complete(Ok(out));
        };

        let sh_abort = shared.clone();
        let on_abort: Rc<dyn Fn(TaskError)> = Rc::new(move |e: TaskError| {
            sh_abort.state.set(match e {
                TaskError::Cancelled => TaskState::Cancelled,
                _ => TaskState::Failed,
            });
            sh_abort.complete(Err(e));
        });
        let sh_state = shared.clone();
        let state_cell: Rc<dyn Fn(TaskState)> = Rc::new(move |s| sh_state.state.set(s));

        shared.state.set(TaskState::Scheduled);
        self.inner.tasks.borrow_mut().insert(
            id,
            TaskEntry {
                fut: Box::pin(wrapped),
                name,
                state: TaskState::Scheduled,
                on_abort,
                state_cell,
            },
        );
        self.inner.run_queue.borrow_mut().push_back(id);

        TaskHandle {
            id,
            shared,
            exec: self.clone(),
        }
    }

    /// Drain the run queue to exhaustion, then return to the simulator.
    /// Port of `EventLoop.run` (cocotb: `_event_loop.py`).
    pub fn run_until_idle(&self) {
        if self.inner.running.get() {
            return; // re-entrant call from within a poll: outer loop continues
        }
        self.inner.running.set(true);
        loop {
            self.drain_woken();
            let next = self.inner.run_queue.borrow_mut().pop_front();
            let Some(id) = next else { break };
            self.poll_task(id);
        }
        self.inner.running.set(false);
    }

    fn drain_woken(&self) {
        let ids: Vec<TaskId> = self.inner.woken.0.lock().unwrap().drain(..).collect();
        for id in ids {
            let mut tasks = self.inner.tasks.borrow_mut();
            if let Some(t) = tasks.get_mut(&id)
                && t.state == TaskState::Pending
            {
                t.state = TaskState::Scheduled;
                (t.state_cell)(TaskState::Scheduled);
                self.inner.run_queue.borrow_mut().push_back(id);
            }
        }
    }

    fn poll_task(&self, id: TaskId) {
        // Take the entry state to Running; leave the entry in the map so
        // handles can query it, but take the future out to poll without
        // holding the borrow.
        let (mut fut, on_abort, state_cell) = {
            let mut tasks = self.inner.tasks.borrow_mut();
            let Some(t) = tasks.get_mut(&id) else { return };
            if t.state != TaskState::Scheduled {
                return; // stale duplicate wake
            }
            t.state = TaskState::Running;
            (t.state_cell)(TaskState::Running);
            // Temporarily replace the future with a no-op placeholder.
            let fut = std::mem::replace(&mut t.fut, Box::pin(async {}));
            (fut, t.on_abort.clone(), t.state_cell.clone())
        };

        let waker = Waker::from(Arc::new(TaskWaker {
            id,
            woken: self.inner.woken.clone(),
        }));
        let mut cx = Context::from_waker(&waker);

        self.inner.currently_polling.set(Some(id));
        let polled = catch_unwind(AssertUnwindSafe(|| fut.as_mut().poll(&mut cx)));
        self.inner.currently_polling.set(None);

        match polled {
            Ok(Poll::Ready(())) => {
                // Wrapper already stored the result and set Finished.
                self.inner.tasks.borrow_mut().remove(&id);
            }
            Ok(Poll::Pending) => {
                let mut tasks = self.inner.tasks.borrow_mut();
                if let Some(t) = tasks.get_mut(&id) {
                    t.fut = fut; // put the future back
                    t.state = TaskState::Pending;
                    (t.state_cell)(TaskState::Pending);
                }
                drop(tasks);
                // Deferred self-cancellation?
                let pending: Vec<TaskId> =
                    self.inner.cancel_pending.borrow_mut().drain(..).collect();
                for cid in pending {
                    self.cancel(cid);
                }
            }
            Err(p) => {
                let msg = panic_message(p);
                let name = self
                    .inner
                    .tasks
                    .borrow()
                    .get(&id)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                self.inner.tasks.borrow_mut().remove(&id);
                drop(fut); // drop the future outside the map borrow
                state_cell(TaskState::Failed);
                on_abort(TaskError::Panicked(msg.clone()));
                if let Some(sink) = self.inner.failure_sink.borrow().as_ref() {
                    sink(&format!("task '{name}' panicked: {msg}"));
                } else {
                    eprintln!("rustdv: unhandled task panic in '{name}': {msg}");
                }
            }
        }
    }

    /// Cancel = drop the future (design-doc §4.6). Cleanup happens in Drop
    /// impls; there is no exception to catch.
    pub fn cancel(&self, id: TaskId) {
        if self.inner.currently_polling.get() == Some(id) {
            self.inner.cancel_pending.borrow_mut().push(id);
            return;
        }
        let entry = self.inner.tasks.borrow_mut().remove(&id);
        if let Some(t) = entry {
            (t.on_abort)(TaskError::Cancelled);
            drop(t.fut);
        }
    }

    /// Cancel every live task with id >= `watermark` (the snapshot taken
    /// *before* the test spawned anything) — the runner uses this to kill
    /// a test's surviving tasks at test end (design-doc §4.5).
    pub fn cancel_after(&self, watermark: TaskId) {
        let ids: Vec<TaskId> = self
            .inner
            .tasks
            .borrow()
            .keys()
            .copied()
            .filter(|&id| id >= watermark)
            .collect();
        for id in ids {
            self.cancel(id);
        }
    }

    /// Current high-water task id (snapshot before starting a test).
    pub fn watermark(&self) -> TaskId {
        self.inner.next_id.get()
    }

    pub fn set_failure_sink(&self, f: Box<dyn Fn(&str)>) {
        *self.inner.failure_sink.borrow_mut() = Some(f);
    }

    pub fn live_tasks(&self) -> usize {
        self.inner.tasks.borrow().len()
    }
}

fn panic_message(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic (non-string payload)".to_string()
    }
}

// ---------------------------------------------------------------------------
// TaskHandle
// ---------------------------------------------------------------------------

/// Public task control surface; also a Future (`handle.await` == awaiting
/// completion, as in cocotb 2.x). Port of cocotb `Task` (design-doc §4.3).
pub struct TaskHandle<T> {
    id: TaskId,
    shared: Rc<TaskShared<T>>,
    exec: Executor,
}

impl<T> Clone for TaskHandle<T> {
    fn clone(&self) -> Self {
        TaskHandle {
            id: self.id,
            shared: self.shared.clone(),
            exec: self.exec.clone(),
        }
    }
}

impl<T> TaskHandle<T> {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn state(&self) -> TaskState {
        self.shared.state.get()
    }

    pub fn done(&self) -> bool {
        matches!(
            self.state(),
            TaskState::Finished | TaskState::Cancelled | TaskState::Failed
        )
    }

    /// Drop-based cancellation (§4.6).
    pub fn cancel(&self) {
        self.exec.cancel(self.id);
    }

    /// The task's result, if complete. Consumes the stored value.
    pub fn result(&self) -> Result<T, TaskError> {
        match self.shared.result.borrow_mut().take() {
            Some(r) => r,
            None => Err(TaskError::InvalidState),
        }
    }
}

impl<T> Future for TaskHandle<T> {
    type Output = Result<T, TaskError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done() {
            return Poll::Ready(self.result());
        }
        self.shared.joiners.borrow_mut().push(cx.waker().clone());
        Poll::Pending
    }
}
