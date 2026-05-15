//! Lifecycle — `WithCallbacks` combinator + `EventReader` poll
//! API. Both surface the same three events (`Started`, `Cycle`,
//! `Completed`); pick whichever shape your host loop prefers.
//!
//! - **Combinator**: `anim.on_complete(|| log!("done"))` returns a
//!   `WithCallbacks<A>` that fires callbacks at the right `t`.
//! - **Event reader**: `let reader = anim.event_reader();` then
//!   `reader.drain()` returns the queued events each frame.
//!   Useful for sync export loops that can't borrow closures.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::Animation;

/// Stable handle issued at registration time. Used by `AnimEvent`
/// so multi-animation drivers can disambiguate fired events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct AnimId(pub u32);

/// Event emitted by a wrapped animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimEvent {
    /// Fired exactly once when the animation transitions from
    /// "haven't sampled yet" to "first sample at t > 0".
    Started(AnimId),
    /// Fired when a cycle boundary passes (every cycle for
    /// `Repeat`-wrapped animations).
    Cycle { id: AnimId, n: u32 },
    /// Fired exactly once when the animation reaches its
    /// terminal duration.
    Completed(AnimId),
}

/// Shared queue of [`AnimEvent`]s. Cheap to clone; one writer,
/// one reader.
#[derive(Clone, Default, Debug)]
pub struct EventReader {
    queue: Rc<RefCell<Vec<AnimEvent>>>,
}

impl EventReader {
    /// Push an event to the queue.
    pub fn push(&self, ev: AnimEvent) {
        self.queue.borrow_mut().push(ev);
    }

    /// Drain all queued events.
    pub fn drain(&self) -> Vec<AnimEvent> {
        std::mem::take(&mut *self.queue.borrow_mut())
    }

    /// Peek the queue length without draining.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.borrow().len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}

/// Combinator wrapping an animation with optional start/complete
/// callbacks + an optional event reader.
pub struct WithCallbacks<A: Animation> {
    inner: A,
    id: AnimId,
    on_start: Option<Box<dyn Fn()>>,
    on_complete: Option<Box<dyn Fn()>>,
    reader: Option<EventReader>,
    /// `RefCell<bool>` lets us emit "started" exactly once even
    /// when the wrapper is sampled many times (animations are
    /// pure-ish; we cheat for lifecycle bookkeeping).
    started: RefCell<bool>,
    completed: RefCell<bool>,
}

impl<A: Animation> WithCallbacks<A> {
    /// Wrap an animation. Default: no callbacks, no reader.
    #[must_use]
    pub fn new(inner: A, id: AnimId) -> Self {
        Self {
            inner,
            id,
            on_start: None,
            on_complete: None,
            reader: None,
            started: RefCell::new(false),
            completed: RefCell::new(false),
        }
    }

    /// Set an on-start callback (fires the first time `sample`
    /// produces a positive `t`).
    #[must_use]
    pub fn on_start(mut self, f: impl Fn() + 'static) -> Self {
        self.on_start = Some(Box::new(f));
        self
    }

    /// Set an on-complete callback (fires when `t ≥ duration`).
    #[must_use]
    pub fn on_complete(mut self, f: impl Fn() + 'static) -> Self {
        self.on_complete = Some(Box::new(f));
        self
    }

    /// Attach an event reader. The same `EventReader` can be
    /// shared across multiple wrapped animations.
    #[must_use]
    pub fn with_reader(mut self, reader: EventReader) -> Self {
        self.reader = Some(reader);
        self
    }
}

impl<A: Animation> Animation for WithCallbacks<A> {
    type Output = A::Output;

    fn duration(&self) -> Duration {
        self.inner.duration()
    }

    fn sample(&self, t: Duration) -> A::Output {
        // Start hook
        if !*self.started.borrow() && t > Duration::ZERO {
            *self.started.borrow_mut() = true;
            if let Some(f) = &self.on_start {
                f();
            }
            if let Some(r) = &self.reader {
                r.push(AnimEvent::Started(self.id));
            }
        }
        // Complete hook
        if !*self.completed.borrow() && t >= self.inner.duration() {
            *self.completed.borrow_mut() = true;
            if let Some(f) = &self.on_complete {
                f();
            }
            if let Some(r) = &self.reader {
                r.push(AnimEvent::Completed(self.id));
            }
        }
        self.inner.sample(t)
    }
}

/// Sugar on the [`Animation`] trait to wrap with `WithCallbacks`.
pub trait AnimationLifecycleExt: Animation + Sized {
    /// Wrap with a `WithCallbacks` carrying the given anim id.
    fn with_callbacks(self, id: AnimId) -> WithCallbacks<Self> {
        WithCallbacks::new(self, id)
    }
}

impl<A: Animation + Sized> AnimationLifecycleExt for A {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LinearRamp;

    #[test]
    fn event_reader_drains() {
        let r = EventReader::default();
        r.push(AnimEvent::Started(AnimId(1)));
        r.push(AnimEvent::Completed(AnimId(1)));
        let drained = r.drain();
        assert_eq!(drained.len(), 2);
        assert!(r.is_empty());
    }

    #[test]
    fn started_fires_once() {
        let r = EventReader::default();
        let wrapped = LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(100))
            .with_callbacks(AnimId(7))
            .with_reader(r.clone());
        let _ = wrapped.sample(Duration::from_millis(10));
        let _ = wrapped.sample(Duration::from_millis(20));
        let _ = wrapped.sample(Duration::from_millis(30));
        let evs = r.drain();
        // 1 Started; no Completed yet.
        assert_eq!(evs.len(), 1);
        assert!(matches!(evs[0], AnimEvent::Started(AnimId(7))));
    }

    #[test]
    fn completed_fires_at_duration() {
        let r = EventReader::default();
        let wrapped = LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(100))
            .with_callbacks(AnimId(0))
            .with_reader(r.clone());
        let _ = wrapped.sample(Duration::from_millis(10));
        let _ = wrapped.sample(Duration::from_millis(120));
        let _ = wrapped.sample(Duration::from_millis(130));
        let evs = r.drain();
        assert_eq!(evs.len(), 2);
        assert!(matches!(evs[0], AnimEvent::Started(_)));
        assert!(matches!(evs[1], AnimEvent::Completed(_)));
    }

    #[test]
    fn closure_callbacks_fire() {
        let started = Rc::new(RefCell::new(false));
        let completed = Rc::new(RefCell::new(false));
        let s = started.clone();
        let c = completed.clone();
        let wrapped = LinearRamp::new(0.0_f32, 1.0, Duration::from_millis(50))
            .with_callbacks(AnimId(0))
            .on_start(move || *s.borrow_mut() = true)
            .on_complete(move || *c.borrow_mut() = true);
        let _ = wrapped.sample(Duration::from_millis(10));
        let _ = wrapped.sample(Duration::from_millis(100));
        assert!(*started.borrow());
        assert!(*completed.borrow());
    }
}
