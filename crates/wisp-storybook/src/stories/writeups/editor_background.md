## Background framing — ED.18

A raw screen recording is a flat rectangle that bleeds to every edge. The
"produced" look every cinematic screen recorder reaches for lifts that
rectangle off a **backdrop**, insets it by a uniform **padding**, and softens
its corners with a **rounded-rect window**. ED.18 is that frame, and — like
the zoom engine — it's applied at compose time, not baked, so the same
function drives the live preview and the export.

The render is two layers, and they map straight onto wisp's
advanced-dispatch path:

- **The backdrop** is a full-NDC `Graphics` rect — a linear gradient (the
  default "Aurora" warm→cool diagonal), a flat color, or (later) a wallpaper
  sprite. It carries no clip, so it draws in **Phase 1**, behind everything.
- **The screen** is the recording sprite with a `MaskShape::RoundedRect`
  clip set to the padded window. The clip makes it a *dispatched* node, so it
  renders to an offscreen target, gets the rounded-corner SDF multiplied into
  its alpha, and composites **over** the backdrop in **Phase 2**.

Because the clip lives in fixed output NDC (screen space, not
transform-aware), the rounded window stays put while the zoom punch-in
(ED.16) tightens *inside* it — the frame is a stable proscenium, the zoom is
the camera move behind it. Padding folds into the same screen transform as a
centered shrink (`scale *= k`, `position *= k`), so it composes exactly with
crop and zoom.

Drop-shadow and the inset border are deferred — a shadow needs a per-frame
offscreen blur pre-pass (and is lavapipe-incompatible), so it lands behind
its own guard rather than slowing the headless gate.
