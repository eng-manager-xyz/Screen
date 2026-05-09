# Status bar — encoding

<iframe src="../../assets/ui/status-bar-busy.html" width="100%" height="48" frameborder="0"></iframe>

`StatusKind::Busy`. Pill swaps to sky-blue with a pulsing dot (reuses
`@keyframes rec-pulse` from the recording toolbar — same motion language
across the app). Detail text reads `Encoding · 38%` next to the pill.

Encoder cell shows live bitrate (`H.264 · 9.4 Mbps`); size cell shows the
file growing (`23.0 MB` after `format_bytes(24_117_248)`). The bytes
formatter rolls through `B → KB → MB → GB` with appropriate fractional
digits — a 1.07 GB recording reads more like a recording than a wall of
digits.

[Open as standalone demo →](../../assets/ui/status-bar-busy.html)

---

[`StatusKind`](../../api/ui_storybook/components/status_bar/enum.StatusKind.html) · [Components index](../components.md)
