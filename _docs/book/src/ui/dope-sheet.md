# UI — dope sheet

The editor's timeline. Tracks are rows (video, cursor, audio, captions,
effects), columns are time, dots are keyframes, the bright vertical line is
the playhead.

A pure presentational component for now — interaction (drag, snap-to-frame,
scrub) lives behind a future signal-driven variant. The visual + structural
contract is locked in `tests/snapshots.rs`.

## Multi-track baseline

<iframe src="../assets/ui/dope-sheet-basic.html" width="100%" height="220" frameborder="0"></iframe>

[Open as live demo →](../assets/ui/dope-sheet-basic.html)

## Dense keyframes

Twelve keyframes alternating ease/linear on a `TrackKind::Effect` zoom track,
in addition to the four standard tracks. Confirms the dot positioning math
holds at high density.

<iframe src="../assets/ui/dope-sheet-dense.html" width="100%" height="280" frameborder="0"></iframe>

[Open as live demo →](../assets/ui/dope-sheet-dense.html)

## Embedded in a card (composition)

The expected production placement: dope sheet wrapped in a `Card` with title
and metadata in the header.

<iframe src="../assets/ui/card-with-dope-sheet.html" width="100%" height="320" frameborder="0"></iframe>

[Open as live demo →](../assets/ui/card-with-dope-sheet.html)

## Keyframe glyphs

| Glyph | `KeyframeKind` | Use |
|---|---|---|
| ◆ (gray) | `Hold` | Hold-until-next |
| ◆ (sky) | `Linear` | Linear interp |
| ◆ (violet) | `Ease` | Ease in/out |
| ▮ (yellow) | `Marker` | Range / chapter / caption marker |
