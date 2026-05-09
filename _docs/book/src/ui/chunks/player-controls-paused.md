# Player controls — paused

<iframe src="../../assets/ui/player-controls-paused.html" width="100%" height="80" frameborder="0"></iframe>

Transport bar at rest. Round play button (`▶`), `0:00 / 1:24` time
display, scrub handle parked at the start.

The component is purely presentational: `position` is a `0.0..=1.0`
fraction the parent owns. `format_time` rounds seconds to the nearest
second so the display ticks once per beat rather than on every frame.

[Open as standalone demo →](../../assets/ui/player-controls-paused.html)

---

[`PlayerControls` API](../../api/ui_storybook/components/player_controls/fn.PlayerControls.html) · [Components index](../components.md)
