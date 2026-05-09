# Recording toolbar — paused

<iframe src="../../assets/ui/recording-toolbar-paused.html" width="100%" height="80" frameborder="0"></iframe>

`Paused`. The dot stops pulsing and switches to the marker-yellow color
(reuses the `--kf-marker` token, which is the same yellow the dope sheet
uses for chapter markers — visual consistency across the app's
"interrupted state" language).

Action stack is `Resume` (primary, red) + `Stop` (outline). The "Resume"
button intentionally uses the same red as the initial "Start recording" —
they're the same action, mechanically: begin/continue capture.

Timer freezes at the elapsed value (`02:17` here) until resume.

[Open as standalone demo →](../../assets/ui/recording-toolbar-paused.html)

---

[`RecordingToolbar`](../../api/ui_storybook/components/recording_toolbar/fn.RecordingToolbar.html) · [Components index](../components.md)
