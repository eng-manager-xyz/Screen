# The zoom engine — ED.16

On an animation stand the camera lived on a column above the artwork, and
the operator pushed in by turning a screw drive — a slow, deliberate move
from wide to tight, hold on the detail, then ease back out. That move, the
**rostrum push-in**, is the single gesture that reads as "cinematic" in a
screen recording: the viewer's eye is walked to exactly the thing that
matters. ED.16 is that move in software — and, crucially, it is *pure
arithmetic*, so the same function drives the live preview and the final
export. What you scrub is what you ship.

```mermaid
flowchart LR
  Z["ZoomSegment\nstart · end · amount · target · ease"] --> F["zoom_at(seg, frame, ramp)"]
  P["project frame"] --> F
  F --> T["ZoomTransform\nscale · center_x · center_y"]
  T --> R["renderer scales the framed\nscreen about the focal point"]
```

A zoom is not stored as baked frames — it's a [`ZoomSegment`](../../api/edit/zoom/struct.ZoomSegment.html)
value, and [`zoom_at`](../../api/edit/zoom_anim/fn.zoom_at.html) recomputes
the transform for any frame on demand. That's the editor's founding rule —
*never cut the negative* — applied to motion: the zoom is a function of the
frame, evaluated at preview and again at export, never a destructive bake.

## The three-phase profile

Over a zoom's `[start, end)` window the scale follows an eased ramp-in to
full `amount`, a flat hold, and a symmetric eased ramp-out back to no-zoom.
The ramp length is clamped to **half the window**, so the two ramps can
never overlap — a very short zoom degrades to a clean triangle (push-in
straight into push-out) rather than fighting itself.

| Phase | Frames (100-frame zoom, ramp 18) | Scale |
|---|---|---|
| ramp-in | `[start, start+18)` | `1.0 → amount`, eased |
| hold | `[start+18, end-18)` | `amount` |
| ramp-out | `[end-18, end)` | `amount → 1.0`, eased |

The easing is the segment's [`EditEase`](../../api/edit/zoom/enum.EditEase.html);
its default, `InOutCubic`, is the "Easy Ease" feel — accelerate off the
wide shot, decelerate into the detail. `EditEase::eval` maps a `0..1` ramp
fraction to eased progress, and every curve is pinned to `f(0)=0, f(1)=1`
so the window's edges always meet no-zoom exactly.

```admonish important title="Focal point is fixed; only scale animates"
The transform scales the frame *about the target point*, and that point is
constant across the whole window — only the scale ramps. Because scale
starts at `1.0` (no visible zoom regardless of where the focal point is),
there's no jump when the window opens; the push-in simply tightens toward
the target. An `Auto` target punches into the centre until click telemetry
(ED.17) resolves it to a real point. `active_zoom_at` walks the project's
zoom list and returns the active window's transform, or identity.
```

## From transform to pixels

The `ZoomTransform` now drives real pixels. `EditorPreview::render_framed`
writes a single **crop-then-zoom** affine into the screen sprite's
transform and composes through the recorder's proven `wisp` path, so the
export generator and the live preview punch in through the *same* code —
preview/export parity by construction. The screen sprite is centre-anchored
at scale 2 (it fills NDC `[-1, 1]`); a `ZoomTransform` of scale `z` becomes
sprite scale `2 · z` with the focal point pinned in place via
`position += focal_ndc · (1 − z)`, where `focal_ndc = (2·fx − 1, −(2·fy − 1))`
— the `−` on `y` is wisp's `+y`-up convention (the decoded top-down frame is
flipped bottom-up at upload). Crop composes underneath: the sub-rect is
pre-scaled `2/w, 2/h` and recentred, then the zoom rides on top.

```admonish warning title="The +y flip is where this goes wrong"
Sign errors in the focal `y` term silently mirror the zoom vertically. The
transform math is unit-tested (centre-2× → scale 4, no shift; a corner zoom
pins the focal edge; a quadrant crop fills the frame; sub-1.0 amounts clamp
to no-zoom) and a golden render asserts a 2× zoom *magnifies* the focal
region ~4× in area — but the flip itself was confirmed **by eye** against a
four-quadrant + centre-marker test pattern before this shipped.
```
