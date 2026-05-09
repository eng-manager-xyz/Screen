# Dope sheet — dense keyframes

<iframe src="../../assets/ui/dope-sheet-dense.html" width="100%" height="280" frameborder="0"></iframe>

Twelve keyframes alternating ease/linear on a `TrackKind::Effect` zoom track,
in addition to the four standard tracks.

Confirms the dot positioning math holds at high density (no overlap or
clipping at the seconds boundary). Also exercises the `track-effect`
visual styling — pink-tinted row background — to validate the per-kind
gradient approach.

The playhead is at `t=5.1s` — not on a frame boundary on purpose, to
verify the floating-point positioning.

[Open as standalone demo →](../../assets/ui/dope-sheet-dense.html)

---

[`DopeSheet` API](../../api/ui_storybook/components/dope_sheet/fn.DopeSheet.html) · [Dope sheet overview](../dope-sheet.md)
