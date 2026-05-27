//! Pointer dispatcher integration tests (WI.2 / AUT-305).
//!
//! Builds a tiny `wisp::Stage` with parent → child nodes, registers
//! handlers via `CallbackRegistry`, drives synthetic raw events
//! through `PointerDispatcher`, and asserts the observed event
//! sequence via interior-mutable counter cells.

use std::cell::Cell;
use std::rc::Rc;

use glam::Vec2;
use wisp::scene::{Container, Stage};
use wisp_interaction::{
    CallbackRegistry, Click, DragStart, Hit, ModifierState, MouseButton, Out, Over, Pointer,
    PointerDispatcher, PointerId, PointerLocation, Press, Release, WheelDelta,
};

fn loc(x: f32, y: f32) -> PointerLocation {
    PointerLocation {
        viewport: Vec2::new(x, y),
        modifiers: ModifierState::none(),
    }
}

fn hit(node: wisp::scene::NodeId) -> Hit {
    Hit {
        node,
        depth: 0.0,
        local_pos: Vec2::ZERO,
    }
}

#[test]
fn press_then_release_on_same_target_emits_click() {
    let mut stage = Stage::new();
    let root = stage.root();
    let n = stage.add_child(root, Container::new()).expect("add_child");

    let mut registry = CallbackRegistry::new();
    let press_count = Rc::new(Cell::new(0_u32));
    let release_count = Rc::new(Cell::new(0_u32));
    let click_count = Rc::new(Cell::new(0_u32));
    {
        let p = press_count.clone();
        let r = release_count.clone();
        let c = click_count.clone();
        registry.on_press(n, move |_e: &Pointer<Press>| p.set(p.get() + 1));
        registry.on_release(n, move |_e: &Pointer<Release>| r.set(r.get() + 1));
        registry.on_click(n, move |_e: &Pointer<Click>| c.set(c.get() + 1));
    }

    let mut d = PointerDispatcher::new();
    let hits = vec![hit(n)];
    d.on_pointer_press(
        PointerId::Mouse,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );
    d.on_pointer_release(
        PointerId::Mouse,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );

    assert_eq!(press_count.get(), 1);
    assert_eq!(release_count.get(), 1);
    assert_eq!(click_count.get(), 1);
}

#[test]
fn press_then_release_outside_does_not_emit_click_but_does_emit_release_to_press_path() {
    let mut stage = Stage::new();
    let root = stage.root();
    let a = stage.add_child(root, Container::new()).expect("a");
    let b = stage.add_child(root, Container::new()).expect("b");

    let mut registry = CallbackRegistry::new();
    let release_a = Rc::new(Cell::new(0_u32));
    let click_a = Rc::new(Cell::new(0_u32));
    let click_b = Rc::new(Cell::new(0_u32));
    {
        let r = release_a.clone();
        let c = click_a.clone();
        let cb = click_b.clone();
        registry.on_release(a, move |_e| r.set(r.get() + 1));
        registry.on_click(a, move |_e| c.set(c.get() + 1));
        registry.on_click(b, move |_e| cb.set(cb.get() + 1));
    }

    let mut d = PointerDispatcher::new();
    // Press over a.
    d.on_pointer_press(
        PointerId::Mouse,
        loc(10.0, 10.0),
        MouseButton::Left,
        &[hit(a)],
        &stage,
        &registry,
    );
    // Move pointer to over b (no longer over a's path).
    d.on_pointer_move(
        PointerId::Mouse,
        loc(50.0, 50.0),
        &[hit(b)],
        &stage,
        &registry,
    );
    // Release over b — not on press path.
    d.on_pointer_release(
        PointerId::Mouse,
        loc(50.0, 50.0),
        MouseButton::Left,
        &[hit(b)],
        &stage,
        &registry,
    );

    // Release fires on press path (a).
    assert_eq!(release_a.get(), 1, "release must fire on the press path");
    // No click anywhere — release was on a different node.
    assert_eq!(click_a.get(), 0, "no click on a — released elsewhere");
    assert_eq!(click_b.get(), 0, "no click on b — pressed elsewhere");
}

#[test]
fn pointer_move_emits_out_then_over_when_target_changes() {
    let mut stage = Stage::new();
    let root = stage.root();
    let a = stage.add_child(root, Container::new()).expect("a");
    let b = stage.add_child(root, Container::new()).expect("b");

    let mut registry = CallbackRegistry::new();
    let log = Rc::new(Cell::new(String::new()));
    {
        let l = log.clone();
        registry.on_over(a, move |_e: &Pointer<Over>| {
            let mut s = l.take();
            s.push_str("over-a;");
            l.set(s);
        });
        let l = log.clone();
        registry.on_out(a, move |_e: &Pointer<Out>| {
            let mut s = l.take();
            s.push_str("out-a;");
            l.set(s);
        });
        let l = log.clone();
        registry.on_over(b, move |_e: &Pointer<Over>| {
            let mut s = l.take();
            s.push_str("over-b;");
            l.set(s);
        });
    }

    let mut d = PointerDispatcher::new();
    d.on_pointer_move(
        PointerId::Mouse,
        loc(10.0, 10.0),
        &[hit(a)],
        &stage,
        &registry,
    );
    d.on_pointer_move(
        PointerId::Mouse,
        loc(50.0, 50.0),
        &[hit(b)],
        &stage,
        &registry,
    );

    let s = log.take();
    assert_eq!(s, "over-a;out-a;over-b;");
}

#[test]
fn multi_touch_two_pointers_independently_dispatch() {
    let mut stage = Stage::new();
    let root = stage.root();
    let n = stage.add_child(root, Container::new()).expect("n");

    let mut registry = CallbackRegistry::new();
    let click_count = Rc::new(Cell::new(0_u32));
    {
        let c = click_count.clone();
        registry.on_click(n, move |_e| c.set(c.get() + 1));
    }

    let mut d = PointerDispatcher::new();
    let hits = vec![hit(n)];
    let p1 = PointerId::Touch(1);
    let p2 = PointerId::Touch(2);

    d.on_pointer_press(
        p1,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );
    d.on_pointer_press(
        p2,
        loc(20.0, 20.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );
    d.on_pointer_release(
        p1,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );
    d.on_pointer_release(
        p2,
        loc(20.0, 20.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );

    // Each touch is a distinct press/release pair → two clicks total.
    assert_eq!(click_count.get(), 2);
}

#[test]
fn drag_threshold_distinguishes_click_from_drag() {
    let mut stage = Stage::new();
    let root = stage.root();
    let n = stage.add_child(root, Container::new()).expect("n");

    let mut registry = CallbackRegistry::new();
    let click_count = Rc::new(Cell::new(0_u32));
    let drag_start_count = Rc::new(Cell::new(0_u32));
    {
        let c = click_count.clone();
        registry.on_click(n, move |_e| c.set(c.get() + 1));
        let d = drag_start_count.clone();
        registry.on_drag_start(n, move |_e: &Pointer<DragStart>| d.set(d.get() + 1));
    }

    let mut d = PointerDispatcher::new();
    let hits = vec![hit(n)];
    d.on_pointer_press(
        PointerId::Mouse,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );
    // Move 10px — over the 5px threshold → drag.
    d.on_pointer_move(PointerId::Mouse, loc(20.0, 10.0), &hits, &stage, &registry);
    d.on_pointer_release(
        PointerId::Mouse,
        loc(20.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );

    assert_eq!(drag_start_count.get(), 1, "10px move must promote to drag");
    assert_eq!(
        click_count.get(),
        0,
        "drag releases emit DragEnd, not Click"
    );
}

#[test]
fn scroll_delta_units_pixel_vs_line_preserved() {
    let mut stage = Stage::new();
    let root = stage.root();
    let n = stage.add_child(root, Container::new()).expect("n");

    let mut registry = CallbackRegistry::new();
    let pixel_count = Rc::new(Cell::new(0_u32));
    let line_count = Rc::new(Cell::new(0_u32));
    {
        let p = pixel_count.clone();
        let l = line_count.clone();
        registry.on_scroll(n, move |e: &Pointer<wisp_interaction::Scroll>| {
            match e.event.delta {
                WheelDelta::Pixel(_) => p.set(p.get() + 1),
                WheelDelta::Line(_) => l.set(l.get() + 1),
            }
        });
    }

    let mut d = PointerDispatcher::new();
    let hits = vec![hit(n)];
    d.on_pointer_scroll(
        PointerId::Mouse,
        loc(10.0, 10.0),
        WheelDelta::Pixel(Vec2::new(0.0, 10.0)),
        &hits,
        &stage,
        &registry,
    );
    d.on_pointer_scroll(
        PointerId::Mouse,
        loc(10.0, 10.0),
        WheelDelta::Line(Vec2::new(0.0, 1.0)),
        &hits,
        &stage,
        &registry,
    );

    assert_eq!(pixel_count.get(), 1);
    assert_eq!(line_count.get(), 1);
}

#[test]
fn stop_bubble_halts_ancestor_dispatch() {
    let mut stage = Stage::new();
    let root = stage.root();
    let parent = stage.add_child(root, Container::new()).expect("parent");
    let child = stage.add_child(parent, Container::new()).expect("child");

    let mut registry = CallbackRegistry::new();
    let child_clicked = Rc::new(Cell::new(0_u32));
    let parent_clicked = Rc::new(Cell::new(0_u32));
    {
        let c = child_clicked.clone();
        registry.on_click(child, move |e: &Pointer<Click>| {
            c.set(c.get() + 1);
            e.stop_bubble();
        });
        let p = parent_clicked.clone();
        registry.on_click(parent, move |_e| p.set(p.get() + 1));
    }

    let mut d = PointerDispatcher::new();
    let hits = vec![hit(child)];
    d.on_pointer_press(
        PointerId::Mouse,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );
    d.on_pointer_release(
        PointerId::Mouse,
        loc(10.0, 10.0),
        MouseButton::Left,
        &hits,
        &stage,
        &registry,
    );

    assert_eq!(child_clicked.get(), 1);
    assert_eq!(
        parent_clicked.get(),
        0,
        "stop_bubble must halt the ancestor dispatch"
    );
}
