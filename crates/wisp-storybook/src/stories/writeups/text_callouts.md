Five callout shapes composed from existing `Graphics` primitives +
`CaptionBlock` / text sprites:

1. **Caption pill** — `CaptionBlock` with a large corner radius and a
   warm fill, for status-style headers.
2. **Number badge** — `Graphics::draw_ellipse` + a small centered
   text texture, for "step 3 of 7" pointers.
3. **Label box** — `CaptionBlock` with the default dark fill, for
   explanatory text bound to a region.
4. **Pointer + label** — line stroke from a label to a small filled
   target circle, for tying a label to an exact pixel.
5. **Arrow + label** — line stroke with a hand-drawn wedge tip
   (three lines), plus a small text caption.

No new wisp primitives — the vocabulary is `draw_rounded_rect`,
`draw_ellipse`, `draw_line`, and the text-texture pipeline.
