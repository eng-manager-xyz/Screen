# Player controls — near end

<iframe src="../../assets/ui/player-controls-near-end.html" width="100%" height="80" frameborder="0"></iframe>

`position=0.94`. Confirms the handle sits inside the track at the
right edge — no overflow past the rounded end-cap, no clipping. The
`margin-left: -6px` on `.player-scrub-handle` exactly cancels the half-width
so the handle's center aligns with the position percentage.

This story exists specifically to lock that boundary. Without it, a
careless tweak to handle dimensions could push the dot off the end of the
track and the SSR snapshot wouldn't catch the visual regression — only
this kind of "edge case" story does.

[Open as standalone demo →](../../assets/ui/player-controls-near-end.html)

---

[`PlayerControls`](../../api/ui_storybook/components/player_controls/fn.PlayerControls.html) · [Components index](../components.md)
