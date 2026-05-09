# Drop zone — active

<iframe src="../../assets/ui/drop-zone-active.html" width="100%" height="280" frameborder="0"></iframe>

The same component, `state=Active`. Solid accent border (sky-blue, the
linear-keyframe color — visually consistent with the editor's "active"
language across the app). Background tint shifts to the same sky at low
alpha. Glyph picks up the accent.

The transform-scale (`1.005`) is intentional — small enough not to feel
animated-for-its-own-sake, big enough to confirm the drag is recognized
under fast cursor motion.

Headline + subtext swap to "Release to import" / "Will open in the editor"
so users get a confirmation of what's about to happen before they let go.

[Open as standalone demo →](../../assets/ui/drop-zone-active.html)

---

[`DropZoneState`](../../api/ui_storybook/components/drop_zone/enum.DropZoneState.html) · [Components index](../components.md)
