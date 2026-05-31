# Video track + clip selection — ED.9

The video lane shows the recording as its **segments** — proportional clip
blocks along the timeline — and lets you select one to edit.

[`segment_spans`](../../api/app_ui/filmstrip/fn.segment_spans.html) is the
pure layout: each [`TimelineSegment`](../../api/edit/segment/struct.TimelineSegment.html)
becomes a `start_fraction` / `width_fraction` of the project, so the lane
tiles responsively at any width. A 2× segment is half as wide as its
source span (it occupies half the project time) — width tracks **project**
length, not source length, which is what keeps the lane in sync with the
ruler after a speed change.

The [`VideoFilmstrip`](../../api/app_ui/filmstrip/fn.VideoFilmstrip.html)
component renders those spans as clip blocks with duration labels; clicking
one sets the selected-clip signal (a `RwSignal<Option<usize>>` in context)
that the inspector (ED.18) and edit operations (ED.11) read. Because the
spans are derived from the segment list every render, the lane re-flows
automatically when a split or trim changes the segments.

```admonish note title="Thumbnails land with render integration"
The clip blocks carry duration labels now; per-clip **thumbnail images**
(decode sample frames through `EditorVideoStream`, CPU-downscale, strip
them across each block) join the render-integration pass alongside the
live preview window — the responsive layout + selection that everything
else hangs off is what this chunk nails down.
```
