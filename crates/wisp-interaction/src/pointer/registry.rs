//! `CallbackRegistry` — closure-on-NodeId callback table.
//!
//! Non-ECS equivalent of Bevy's targeted observers
//! (`entity.observe(|e: On<Pointer<Click>>| ...)`). Consumers
//! register `Box<dyn Fn(&Pointer<E>)>` keyed by `(NodeId, EventKind)`;
//! the dispatcher looks up + invokes during bubble walk.

use std::collections::HashMap;

use crate::pointer::events::{
    Cancel, Click, Drag, DragDrop, DragEnd, DragEnter, DragLeave, DragOver, DragStart, EventKind,
    Move, Out, Over, Pointer, Press, Release, Scroll,
};
use wisp::scene::NodeId;

/// Type-erased callback. Each variant boxes the appropriate
/// `Fn(&Pointer<E>)` so the dispatcher can call without downcasting.
///
/// Storing one boxed closure per (`NodeId`, `EventKind`) is the simplest
/// shape; a more polished follow-up could `Vec<Box<...>>` to support
/// multiple handlers per (node, kind). For v1 a registered handler
/// REPLACES the previous one — matches the "one observer per kind"
/// convention from Bevy's `entity.observe(...)`.
#[allow(
    clippy::type_complexity,
    reason = "Each variant is a boxed closure with a different argument type; aliasing would only hide the same shape and lose IDE-completion value."
)]
pub enum Callback {
    /// `Pointer<Over>` handler.
    Over(Box<dyn Fn(&Pointer<Over>)>),
    /// `Pointer<Out>` handler.
    Out(Box<dyn Fn(&Pointer<Out>)>),
    /// `Pointer<Move>` handler.
    Move(Box<dyn Fn(&Pointer<Move>)>),
    /// `Pointer<Cancel>` handler.
    Cancel(Box<dyn Fn(&Pointer<Cancel>)>),
    /// `Pointer<Press>` handler.
    Press(Box<dyn Fn(&Pointer<Press>)>),
    /// `Pointer<Release>` handler.
    Release(Box<dyn Fn(&Pointer<Release>)>),
    /// `Pointer<Click>` handler.
    Click(Box<dyn Fn(&Pointer<Click>)>),
    /// `Pointer<DragStart>` handler.
    DragStart(Box<dyn Fn(&Pointer<DragStart>)>),
    /// `Pointer<Drag>` handler.
    Drag(Box<dyn Fn(&Pointer<Drag>)>),
    /// `Pointer<DragEnd>` handler.
    DragEnd(Box<dyn Fn(&Pointer<DragEnd>)>),
    /// `Pointer<DragEnter>` handler.
    DragEnter(Box<dyn Fn(&Pointer<DragEnter>)>),
    /// `Pointer<DragOver>` handler.
    DragOver(Box<dyn Fn(&Pointer<DragOver>)>),
    /// `Pointer<DragLeave>` handler.
    DragLeave(Box<dyn Fn(&Pointer<DragLeave>)>),
    /// `Pointer<DragDrop>` handler.
    DragDrop(Box<dyn Fn(&Pointer<DragDrop>)>),
    /// `Pointer<Scroll>` handler.
    Scroll(Box<dyn Fn(&Pointer<Scroll>)>),
}

impl Callback {
    /// Which `EventKind` this callback responds to. Useful for the
    /// registry's secondary key.
    #[must_use]
    pub fn kind(&self) -> EventKind {
        match self {
            Self::Over(_) => EventKind::Over,
            Self::Out(_) => EventKind::Out,
            Self::Move(_) => EventKind::Move,
            Self::Cancel(_) => EventKind::Cancel,
            Self::Press(_) => EventKind::Press,
            Self::Release(_) => EventKind::Release,
            Self::Click(_) => EventKind::Click,
            Self::DragStart(_) => EventKind::DragStart,
            Self::Drag(_) => EventKind::Drag,
            Self::DragEnd(_) => EventKind::DragEnd,
            Self::DragEnter(_) => EventKind::DragEnter,
            Self::DragOver(_) => EventKind::DragOver,
            Self::DragLeave(_) => EventKind::DragLeave,
            Self::DragDrop(_) => EventKind::DragDrop,
            Self::Scroll(_) => EventKind::Scroll,
        }
    }
}

/// Closure-on-NodeId registry. Hosts use [`on_click`](Self::on_click)
/// etc. to attach callbacks; the [`PointerDispatcher`] looks them up
/// during bubble walk and invokes.
#[derive(Default)]
pub struct CallbackRegistry {
    table: HashMap<(NodeId, EventKind), Callback>,
}

impl CallbackRegistry {
    /// Construct empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered callbacks. For tests + diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// `true` when no callbacks registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Register an arbitrary callback.
    pub fn register(&mut self, node: NodeId, cb: Callback) {
        let kind = cb.kind();
        self.table.insert((node, kind), cb);
    }

    /// Drop a registered callback. No-op if not registered.
    pub fn unregister(&mut self, node: NodeId, kind: EventKind) {
        self.table.remove(&(node, kind));
    }

    /// Drop every callback for this node. Called by the host on
    /// destroy-node cleanup.
    pub fn unregister_all(&mut self, node: NodeId) {
        self.table.retain(|(n, _), _| *n != node);
    }

    /// Lookup helper used by the dispatcher.
    pub(crate) fn get(&self, node: NodeId, kind: EventKind) -> Option<&Callback> {
        self.table.get(&(node, kind))
    }

    // Convenience constructors — keep parity with the Linear spec's
    // ergonomic surface (`registry.on_click(node, |e| ...)`).

    /// Register a click handler.
    pub fn on_click(&mut self, node: NodeId, f: impl Fn(&Pointer<Click>) + 'static) {
        self.register(node, Callback::Click(Box::new(f)));
    }

    /// Register an over (hover-enter) handler.
    pub fn on_over(&mut self, node: NodeId, f: impl Fn(&Pointer<Over>) + 'static) {
        self.register(node, Callback::Over(Box::new(f)));
    }

    /// Register an out (hover-leave) handler.
    pub fn on_out(&mut self, node: NodeId, f: impl Fn(&Pointer<Out>) + 'static) {
        self.register(node, Callback::Out(Box::new(f)));
    }

    /// Register a press (button-down) handler.
    pub fn on_press(&mut self, node: NodeId, f: impl Fn(&Pointer<Press>) + 'static) {
        self.register(node, Callback::Press(Box::new(f)));
    }

    /// Register a release (button-up) handler.
    pub fn on_release(&mut self, node: NodeId, f: impl Fn(&Pointer<Release>) + 'static) {
        self.register(node, Callback::Release(Box::new(f)));
    }

    /// Register a drag-start handler.
    pub fn on_drag_start(&mut self, node: NodeId, f: impl Fn(&Pointer<DragStart>) + 'static) {
        self.register(node, Callback::DragStart(Box::new(f)));
    }

    /// Register a drag (motion-while-pressed) handler.
    pub fn on_drag(&mut self, node: NodeId, f: impl Fn(&Pointer<Drag>) + 'static) {
        self.register(node, Callback::Drag(Box::new(f)));
    }

    /// Register a drag-end handler.
    pub fn on_drag_end(&mut self, node: NodeId, f: impl Fn(&Pointer<DragEnd>) + 'static) {
        self.register(node, Callback::DragEnd(Box::new(f)));
    }

    /// Register a scroll handler.
    pub fn on_scroll(&mut self, node: NodeId, f: impl Fn(&Pointer<Scroll>) + 'static) {
        self.register(node, Callback::Scroll(Box::new(f)));
    }
}
