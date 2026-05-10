# Text + filter + blend composition (M-TEXT.6 / AUT-80)

Three pieces of text on a single warm backdrop, demonstrating the
acceptance surfaces of M-TEXT.6:

- **Normal** — baseline composition over a sprite backdrop.
- **Subtract** — the same white glyphs with `BlendMode::Subtract` on
  the sprite's container: `dst - src` clamped, so the warm backdrop
  is punched out to near-black where the glyph alpha is high.
  (Multiply with white text against the warm backdrop would give back
  the backdrop color — invisible — so we chose Subtract for visual
  punch. Both go through the same `BlendMode` enum on `Container`.)
- **Filtered** — the source text is rendered in saturated orange, then
  routed through `Renderer::apply_filter(ColorMatrixFilter::grayscale(),
  …)` before being attached as a sprite. The hue is gone in the output.

## What this story proves

| Acceptance criterion | Demonstration |
|---|---|
| Renders through `render_stage` | All sprites in this story go through the standard pipeline. |
| Renders offscreen | The "Filtered" path renders text into one `RenderTexture`, applies the filter to another, then samples that. |
| Normal + one non-Normal blend mode | Normal + Multiply, side by side. |
| Filtered text | Grayscale color-matrix over the orange text. |
| Headless output includes text | Storybook's exporter is exactly the headless export path used here. |

The bitmap output of this story is the chapter hero.
