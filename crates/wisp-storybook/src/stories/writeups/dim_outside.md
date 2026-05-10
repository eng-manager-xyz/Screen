# Dim outside (variants) — M-MASK.7 / AUT-29

`DimOutside` + `DimStrength` are the renderer-data API for the
spotlight-style focus effect. `DimStrength` snaps between named
presets — Light / Medium / Heavy — with a `Custom(f32)` escape hatch
clamped to `[0, 1]`.

This story renders the same screen-capture backdrop three times,
focus-zoned with each named strength, side by side. From left to
right: Light, Medium, Heavy. The legibility of the surrounding grid
shows the dim is actually working — Heavy is nearly black, Light
still shows the structure.

The data wrapper exists for the same reason as `BlurStrength`: the
editor's "dim strength" slider should snap between named cinematic
levels (so retuning Heavy from 0.9 → 0.95 later doesn't break
project files), but `Custom(f32)` keeps deterministic story snapshots
possible.
