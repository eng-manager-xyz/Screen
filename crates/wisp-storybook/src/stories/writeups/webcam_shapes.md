# Webcam shapes — M-MASK.8 / AUT-30

`MaskShape::Circle { center, radius }` joins `Rect` and `RoundedRect`
in the mask shape catalog. Webcam overlays now have two cinematic
options out of the box:

- **Circle** (left) — creator-style YouTube/Twitch overlay.
- **Rounded rectangle** (right) — professional walkthrough framing.

Both are just data — different `MaskShape` enum variants — so the
editor can swap between them without re-rendering, and the
`apply_clip` / `apply_privacy_blur` / `apply_solid_redaction` /
`apply_spotlight` primitives all accept the new variant
automatically. No new shader, no new pipeline.

The story renders a "webcam frame" sprite (tone gradient + grid) into
a render texture, runs `apply_clip` with each shape, and drops the
two clipped textures over a dark gradient backdrop so the silhouettes
are clearly visible.
