# Status bar — error

<iframe src="../../assets/ui/status-bar-error.html" width="100%" height="48" frameborder="0"></iframe>

`StatusKind::Error`. Pill goes red, dot stops pulsing (the user shouldn't
mistake an error for in-progress work). FPS reads `0` because the
renderer is no longer ticking; encoder cell shows the codec without a
live bitrate; size cell freezes at the last known value. Detail text
carries the actual error — `VideoToolbox: out of memory` here.

Whatever appears in `detail` is the only specific information the user
gets at this layer; a longer error log lives in the encoder card (a
future chunk). The status bar is a glanceable summary, not a debug pane.

[Open as standalone demo →](../../assets/ui/status-bar-error.html)

---

[`StatusBar`](../../api/ui_storybook/components/status_bar/fn.StatusBar.html) · [Components index](../components.md)
