//! Animation triggers — convenience wrapper around
//! [`CallbackRegistry`](crate::CallbackRegistry) that wires pointer
//! events to host-supplied callbacks.
//!
//! ## Why this lives here, not in `wisp-animation`
//!
//! The original plan (AUT-311 Linear ticket) called for
//! `Tween::on_click_of(node_id)` directly on `wisp-animation::Tween`.
//! That requires `wisp-animation → wisp-interaction` in the dep graph,
//! which would force every animation consumer to take an input-handling
//! dependency. We flip the direction: animation triggers live HERE as
//! a thin glue layer, the user supplies the action as a `Fn()`
//! closure that closes over whatever `&mut Driver` they own. No
//! `wisp-animation` dep needed in `wisp-interaction`.
//!
//! ## Anti-spam — [`Cooldown`]
//!
//! Click-spamming a button that triggers a 600 ms tween restarts the
//! tween mid-flight, which looks janky. Wrap your action in a
//! [`Cooldown`] to debounce: it'll drop calls that arrive within
//! `interval_secs` of the last accepted one. Reference cadence: a
//! 300 ms cooldown matches Material Design's tap-feedback default.

use std::cell::Cell;
use std::rc::Rc;

use crate::pointer::{
    CallbackRegistry, Click, DragEnd, DragStart, Out, Over, Pointer, Press, Release,
};
use wisp::scene::NodeId;

/// Wraps a no-arg callback with a minimum-interval gate. Wall-clock
/// time is the caller's job — pass `now_secs` into [`Cooldown::try_fire`]
/// (most likely the host's monotonic clock or driver time).
pub struct Cooldown<F: Fn()> {
    action: F,
    interval: f32,
    last_fired: Cell<Option<f32>>,
}

impl<F: Fn()> Cooldown<F> {
    /// Wrap `action` so it can only fire once per `interval_secs`.
    pub fn new(interval_secs: f32, action: F) -> Self {
        Self {
            action,
            interval: interval_secs,
            last_fired: Cell::new(None),
        }
    }

    /// Fire the action if `interval_secs` has elapsed since the last
    /// successful fire (or if this is the first call). Returns
    /// `true` iff the action fired.
    pub fn try_fire(&self, now_secs: f32) -> bool {
        let ok = match self.last_fired.get() {
            None => true,
            Some(last) => now_secs - last >= self.interval,
        };
        if ok {
            (self.action)();
            self.last_fired.set(Some(now_secs));
        }
        ok
    }
}

/// Sugar over [`CallbackRegistry`] for "wire pointer events to
/// no-arg actions". Use directly if your callbacks are simple
/// `Fn()`; drop down to the registry directly for full
/// `Pointer<E>` access.
pub struct AnimationTriggers<'r> {
    registry: &'r mut CallbackRegistry,
}

impl<'r> AnimationTriggers<'r> {
    /// Borrow a registry. Stays alive only as long as you hold the
    /// wrapper.
    pub fn new(registry: &'r mut CallbackRegistry) -> Self {
        Self { registry }
    }

    /// Fire `action` when `node` is clicked (Pointer<Click>).
    pub fn on_click(&mut self, node: NodeId, action: impl Fn() + 'static) {
        self.registry
            .on_click(node, move |_e: &Pointer<Click>| action());
    }

    /// Fire `action` when the pointer enters `node` (Pointer<Over>).
    pub fn on_hover_enter(&mut self, node: NodeId, action: impl Fn() + 'static) {
        self.registry
            .on_over(node, move |_e: &Pointer<Over>| action());
    }

    /// Fire `action` when the pointer leaves `node` (Pointer<Out>).
    pub fn on_hover_leave(&mut self, node: NodeId, action: impl Fn() + 'static) {
        self.registry
            .on_out(node, move |_e: &Pointer<Out>| action());
    }

    /// Fire `action_press` on press, `action_release` on release.
    /// Useful for "press-and-hold" UI patterns (mute / unmute,
    /// momentary buttons).
    pub fn on_press_release(
        &mut self,
        node: NodeId,
        action_press: impl Fn() + 'static,
        action_release: impl Fn() + 'static,
    ) {
        self.registry
            .on_press(node, move |_e: &Pointer<Press>| action_press());
        self.registry
            .on_release(node, move |_e: &Pointer<Release>| action_release());
    }

    /// Fire `action_start` on drag-start, `action_end` on drag-end.
    pub fn on_drag(
        &mut self,
        node: NodeId,
        action_start: impl Fn() + 'static,
        action_end: impl Fn() + 'static,
    ) {
        self.registry
            .on_drag_start(node, move |_e: &Pointer<DragStart>| action_start());
        self.registry
            .on_drag_end(node, move |_e: &Pointer<DragEnd>| action_end());
    }
}

/// Convenience: wrap an `Rc<Cooldown<_>>` action so the same gate
/// fires both directly and via the registry. The closure is
/// `'static` so it can be passed to a registration call without
/// fighting borrow.
pub fn cooldown_action<F: Fn() + 'static>(
    cooldown: Rc<Cooldown<F>>,
    now_secs: impl Fn() -> f32 + 'static,
) -> impl Fn() + 'static {
    move || {
        let _ = cooldown.try_fire(now_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::pointer::{PointerDispatcher, PointerId, PointerLocation};
    use crate::{HitShape, MouseButton, PickableMap, Wisp2dHitTest};
    use wisp::math::Rect;
    use wisp::scene::transform::Transform;
    use wisp::scene::{Container, Stage};

    use crate::HitTestBackend;

    fn container_at(x: f32, y: f32) -> Container {
        let mut c = Container::new();
        c.transform = Transform {
            position: Vec2::new(x, y),
            scale: Vec2::splat(1.0),
            rotation: 0.0,
            pivot: Vec2::ZERO,
            skew: Vec2::ZERO,
        };
        c
    }

    fn modifiers() -> crate::ModifierState {
        crate::ModifierState::none()
    }

    #[test]
    fn cooldown_fires_first_call_then_gates_until_interval_elapses() {
        let fired = Rc::new(Cell::new(0_u32));
        let f = fired.clone();
        let c = Cooldown::new(0.5, move || f.set(f.get() + 1));

        assert!(c.try_fire(0.0));
        assert_eq!(fired.get(), 1);
        // Within interval — gated.
        assert!(!c.try_fire(0.2));
        assert_eq!(fired.get(), 1);
        // Just outside interval — fires.
        assert!(c.try_fire(0.5));
        assert_eq!(fired.get(), 2);
    }

    #[test]
    fn triggers_on_click_fires_via_dispatcher() {
        let mut stage = Stage::new();
        let root = stage.root();
        let n = stage.add_child(root, container_at(0.0, 0.0)).unwrap();
        let mut pickable = PickableMap::new();
        pickable.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));
        let backend = Wisp2dHitTest::new(&stage, &pickable);

        let mut registry = CallbackRegistry::new();
        let fired = Rc::new(Cell::new(0_u32));
        {
            let f = fired.clone();
            let mut t = AnimationTriggers::new(&mut registry);
            t.on_click(n, move || f.set(f.get() + 1));
        }

        let mut d = PointerDispatcher::new();
        let loc = PointerLocation {
            viewport: Vec2::new(10.0, 10.0),
            modifiers: modifiers(),
        };
        let hits = backend.pick(loc.viewport);
        assert_eq!(hits.len(), 1, "sanity: hit-test finds the node");

        d.on_pointer_press(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        d.on_pointer_release(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        assert_eq!(fired.get(), 1);
    }

    #[test]
    fn triggers_on_hover_enter_leave_fire_in_order() {
        let mut stage = Stage::new();
        let root = stage.root();
        let n = stage.add_child(root, container_at(0.0, 0.0)).unwrap();
        let mut pickable = PickableMap::new();
        pickable.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));
        let backend = Wisp2dHitTest::new(&stage, &pickable);

        let mut registry = CallbackRegistry::new();
        let log = Rc::new(Cell::new(String::new()));
        {
            let l = log.clone();
            let mut t = AnimationTriggers::new(&mut registry);
            t.on_hover_enter(n, move || {
                let mut s = l.take();
                s.push_str("enter;");
                l.set(s);
            });
            let l = log.clone();
            let mut t = AnimationTriggers::new(&mut registry);
            t.on_hover_leave(n, move || {
                let mut s = l.take();
                s.push_str("leave;");
                l.set(s);
            });
        }

        let mut d = PointerDispatcher::new();
        let in_loc = PointerLocation {
            viewport: Vec2::new(10.0, 10.0),
            modifiers: modifiers(),
        };
        let out_loc = PointerLocation {
            viewport: Vec2::new(100.0, 100.0),
            modifiers: modifiers(),
        };
        d.on_pointer_move(
            PointerId::Mouse,
            in_loc,
            &backend.pick(in_loc.viewport),
            &stage,
            &registry,
        );
        d.on_pointer_move(
            PointerId::Mouse,
            out_loc,
            &backend.pick(out_loc.viewport),
            &stage,
            &registry,
        );

        assert_eq!(log.take(), "enter;leave;");
    }

    #[test]
    fn triggers_press_release_callbacks_distinct() {
        let mut stage = Stage::new();
        let root = stage.root();
        let n = stage.add_child(root, container_at(0.0, 0.0)).unwrap();
        let mut pickable = PickableMap::new();
        pickable.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));
        let backend = Wisp2dHitTest::new(&stage, &pickable);

        let mut registry = CallbackRegistry::new();
        let press = Rc::new(Cell::new(0_u32));
        let release = Rc::new(Cell::new(0_u32));
        {
            let p = press.clone();
            let r = release.clone();
            let mut t = AnimationTriggers::new(&mut registry);
            t.on_press_release(n, move || p.set(p.get() + 1), move || r.set(r.get() + 1));
        }

        let mut d = PointerDispatcher::new();
        let loc = PointerLocation {
            viewport: Vec2::new(10.0, 10.0),
            modifiers: modifiers(),
        };
        let hits = backend.pick(loc.viewport);
        d.on_pointer_press(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        assert_eq!(press.get(), 1);
        assert_eq!(release.get(), 0);
        d.on_pointer_release(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        assert_eq!(press.get(), 1);
        assert_eq!(release.get(), 1);
    }

    #[test]
    fn cooldown_action_helper_round_trips_through_dispatcher() {
        let mut stage = Stage::new();
        let root = stage.root();
        let n = stage.add_child(root, container_at(0.0, 0.0)).unwrap();
        let mut pickable = PickableMap::new();
        pickable.insert_shape(n, HitShape::Rect(Rect::new(0.0, 0.0, 50.0, 50.0)));
        let backend = Wisp2dHitTest::new(&stage, &pickable);

        let mut registry = CallbackRegistry::new();
        let fired = Rc::new(Cell::new(0_u32));
        let now = Rc::new(Cell::new(0.0_f32));
        {
            let f = fired.clone();
            let cooldown = Rc::new(Cooldown::new(0.5, move || f.set(f.get() + 1)));
            let n_clock = now.clone();
            let action = cooldown_action(cooldown, move || n_clock.get());
            let mut t = AnimationTriggers::new(&mut registry);
            t.on_click(n, action);
        }

        let mut d = PointerDispatcher::new();
        let loc = PointerLocation {
            viewport: Vec2::new(10.0, 10.0),
            modifiers: modifiers(),
        };
        let hits = backend.pick(loc.viewport);

        // First click at t=0 fires.
        now.set(0.0);
        d.on_pointer_press(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        d.on_pointer_release(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        assert_eq!(fired.get(), 1);

        // Second click at t=0.2 is gated (interval is 0.5).
        now.set(0.2);
        d.on_pointer_press(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        d.on_pointer_release(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        assert_eq!(fired.get(), 1);

        // Third click at t=0.6 fires (≥ 0.5 after last fire at 0.0).
        now.set(0.6);
        d.on_pointer_press(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        d.on_pointer_release(
            PointerId::Mouse,
            loc,
            MouseButton::Left,
            &hits,
            &stage,
            &registry,
        );
        assert_eq!(fired.get(), 2);
    }
}
