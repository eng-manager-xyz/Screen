# Zoom push-in — the rostrum move (ED.16)

The single gesture that reads as *cinematic* in a screen recording: a slow,
deliberate push from the wide shot in toward the thing that matters, a hold on
the detail, then an ease back out. This story animates that move over a mock
app card, tightening the camera onto the accent button.

The motion follows the editor's **three-phase profile** — an eased ramp-in to
the peak zoom, a flat hold, and a symmetric eased ramp-out (in-out cubic, the
"Easy Ease" feel). It is reproduced inline here so the story stays a pure
`wisp` demo; in the app the very same shape comes from
`edit::zoom_anim::zoom_at`, evaluated per frame at both preview and export
(*never a destructive bake*).

The load-bearing decision is the **focal pin**: the content node scales about
the focal point, not the frame centre, so the button stays glued in place
while only the scale animates:

```
position = focal · (1 − z)
```

which is exactly how `EditorPreview::render_framed` pins the focal point in the
editor (`position += focal_ndc · (1 − z)`). Because the zoom starts at `z = 1`,
there is no jump when the move opens — the push-in simply tightens toward the
target.
