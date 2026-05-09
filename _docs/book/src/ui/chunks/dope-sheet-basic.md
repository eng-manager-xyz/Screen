# Dope sheet — multi-track

<iframe src="../../assets/ui/dope-sheet-basic.html" width="100%" height="220" frameborder="0"></iframe>

The editor's timeline. Tracks are rows (Video / Cursor / Audio / Caption),
columns are time, dots are keyframes, and the bright vertical line is the
playhead at `t=3.4s`.

A pure presentational component for now — interaction (drag, snap-to-frame,
scrub) lives behind a future signal-driven variant. The visual + structural
contract is locked in `tests/snapshots.rs`.

The keyframe glyph maps to `KeyframeKind`:

| Glyph | Kind | Use |
|---|---|---|
| ◆ gray | `Hold` | Hold-until-next |
| ◆ sky | `Linear` | Linear interpolation |
| ◆ violet | `Ease` | Ease in/out |
| ▮ yellow | `Marker` | Range / chapter / caption marker |

[Open as standalone demo →](../../assets/ui/dope-sheet-basic.html)

---

[`DopeSheet` API](../../api/ui_storybook/components/dope_sheet/fn.DopeSheet.html) · [Dope sheet overview](../dope-sheet.md)
