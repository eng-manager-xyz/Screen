Text used as an alpha mask — `Renderer::apply_mask_to_texture`
takes a foreground RT and an alpha-coverage RT (here, the rendered
text texture), producing a foreground clipped to the text shape.

Three foregrounds:

- **Fill** — saturated horizontal color bands. Text reveals the
  poster underneath.
- **Blur** — three overlapping circles run through `BlurFilter`. Text
  becomes a soft-focus window onto the colored backdrop.
- **Spotlight** — a warm radial highlight. Text glows with the
  recovered warm tones inside.

No new wisp code — `apply_mask_to_texture` was already the load-
bearing primitive that M-VEC.4..6 and M-MASK.2..4 use. Text just
joins the list of valid mask sources.
