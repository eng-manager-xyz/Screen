Seven curated `TextPreset` styles in one gallery: each row is the
preset's name rendered in its own style.

The presets are pure data — `WispTextStyle` values with no allocation
or GPU dependency. The app, editor, and renderer all share the same
constants, so a caption you author in the editor matches the caption
the renderer composes into an export.
