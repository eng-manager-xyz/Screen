# Recording toolbar — idle

<iframe src="../../assets/ui/recording-toolbar-idle.html" width="100%" height="80" frameborder="0"></iframe>

The first surface a user sees when they open the recorder. Status reads
"Ready", timer is `00:00`, source picker shows the currently-selected
display, and a single primary "Start recording" button (red, with a static
white dot to telegraph what's about to happen) takes most of the visual
weight on the right.

This state is intentionally one-button. We don't want a "Pause" or "Stop"
button visible before recording starts — they'd be muted and dead, which
is worse than absent.

[Open as standalone demo →](../../assets/ui/recording-toolbar-idle.html)

---

[`RecordingToolbar` API](../../api/ui_storybook/components/recording_toolbar/fn.RecordingToolbar.html) · [Components index](../components.md)
