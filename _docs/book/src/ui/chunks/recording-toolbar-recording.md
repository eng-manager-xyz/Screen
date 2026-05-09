# Recording toolbar — recording

<iframe src="../../assets/ui/recording-toolbar-recording.html" width="100%" height="80" frameborder="0"></iframe>

State swap to `Recording`. The dot turns red and pulses (CSS keyframes,
~1.4s loop), the status label colors red, the timer ticks (`02:17` here
for `elapsed_seconds=137.0`), and the action stack swaps to
`Pause` (secondary) + `Stop` (outline).

The pulsing dot is the primary recording-is-on signal. It's visible
peripherally — even when the user has the toolbar in the corner of their
eye, the motion confirms capture.

The timer formats as `M:SS` until the hour, then `H:MM:SS`. Recordings
longer than an hour are unusual but the format handles them cleanly.

[Open as standalone demo →](../../assets/ui/recording-toolbar-recording.html)

---

[`RecordingState`](../../api/ui_storybook/components/recording_toolbar/enum.RecordingState.html) · [Components index](../components.md)
