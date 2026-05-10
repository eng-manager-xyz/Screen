# Examples gallery — M-VEC.12 / AUT-64

Single-canvas overview of the M-VEC catalog. Every individual
storybook entry (clip-rounded, mask-texture, vector-render,
vector-overlays, path-stroke, mask-combine, etc.) deep-dives one
primitive; this gallery shows the whole stack in one glance.

| Region | Primitive |
|---|---|
| Top row (4 tiles) | `VectorShape::Rect / RoundedRect / Circle / Ellipse` with solid fills. |
| Top right | `Highlight::outline` — yellow ring around a button. |
| Middle | `Callout::arrow_to` + `PathBuilder::quad_to` — straight arrow + Bezier curve. |
| Lower row | `Callout::label_box` with stroke, `Callout::badge`, `Highlight::pill`. |
| Bottom | `Callout::caption_pill`. |

Use this as the entry point when reviewing the M-VEC track —
glance to confirm the catalog renders, then drill into a chunk
chapter for details on a specific primitive.
