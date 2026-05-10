# Highlight + callout overlays — M-VEC.8 / M-VEC.9

`Highlight` (M-VEC.8) and `Callout` (M-VEC.9) are preset
constructors for the most common attention-guiding overlays. They
produce plain `Vector`s — same data type as the rest of the M-VEC
catalog — so they can be transformed, masked, or composed exactly
like any other vector primitive.

The story renders six presets in a single composition:

- `Highlight::outline(rounded_rect, yellow, 0.025)` — ring around a
  "button."
- `Highlight::pill(rect, cyan, 0.5)` — translucent fill over a label.
- `Callout::label_box(rect, amber, white_stroke, 0.04)` — annotation
  card.
- `Callout::badge(pos, 0.085, red)` — numbered step marker.
- `Callout::caption_pill(rect, dark)` — bottom caption.
- `Highlight::glow(circle, red, 0.05)` — soft-glow approximation
  (preliminary; true Gaussian glow lands with M-DYN.7 feathering).

The `Highlight::glow` is documented as a placeholder — a wider stroke
with low alpha. M-DYN.7 (AUT-49 P2) will add feathered edges, at
which point glow becomes a real Gaussian falloff.
