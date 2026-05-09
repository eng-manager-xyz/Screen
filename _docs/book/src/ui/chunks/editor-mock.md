# Editor mock — full composition

<iframe src="../../assets/ui/editor-mock.html" width="100%" height="640" frameborder="0"></iframe>

The whole editor mock in one story: `Card` (Recording 02 metadata) →
preview placeholder (gradient surface) → `PlayerControls` (playing,
mid-clip) → second `Card` (Timeline) → `DopeSheet` with the standard four
tracks.

This is the reference composition the Tauri shell will mount once the
Leptos integration lands (M-INT.1). Three things it locks:

1. **Vertical rhythm.** Card padding + 12px scrub gap + 14px between cards
   adds up to a comfortable density without scroll on a 720p editor view.
2. **Color reuse.** The preview gradient picks up the same sky/violet
   accents the dope-sheet keyframes use, so the eye reads the editor as
   one palette.
3. **Component boundaries.** `Card` is the only "chrome" — both the player
   surface and the timeline live inside their own `Card`, which is the
   pattern the rest of the editor will follow (settings panel, captions
   panel, export panel).

[Open as standalone demo →](../../assets/ui/editor-mock.html)

---

[Components index](../components.md) · [Dope sheet](../dope-sheet.md)
