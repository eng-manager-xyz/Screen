# Animated click ripple — M0.13

![click ripple](../../assets/wisp/graphics-ellipse.png)

Three click ripples animated via the story's `tick` hook.

Each ripple is one outlined ellipse: filled with a low-alpha center, stroked
with a brighter outer edge that fades as the radius grows. Three are
staggered in time to read as separate clicks.

The ellipse SDF uses the standard scaled-circle approximation:
`(length(p / r) - 1.0) * min(r.x, r.y)`. Visually correct for moderate
eccentricities; exact ellipse SDF requires iteration and isn't necessary for
ripple effects.

This is exactly the recorder's click-ripple feature (M0.18+ in the recorder
roadmap): captured cursor events trigger ellipse animations on the timeline,
each a few hundred ms long. The renderer batches every ripple in the frame
into one draw call.

---

[`Graphics` API](../../api/wisp/scene/struct.Graphics.html) · [Stories index](../stories.md)
