# Timeline skeleton

[Linear: AUT-139](https://linear.app/harwood/issue/AUT-139)

Layout-only timeline scaffold — transport row + per-track labels +
dashed placeholder content. The real keyframe editing lives in
`DopeSheet`; this skeleton is what the editor renders below the
canvas when no clip is loaded or when a track has nothing on it.

<iframe src="../../assets/ui/timeline-with-placeholders.html" width="980" height="220" frameborder="0"></iframe>

## States

| State | Story |
| --- | --- |
| Empty (no placeholders) | [`timeline-empty`](../../assets/ui/timeline-empty.html) |
| With placeholders | [`timeline-with-placeholders`](../../assets/ui/timeline-with-placeholders.html) |
| Playing | [`timeline-playing`](../../assets/ui/timeline-playing.html) |
| Selected video track | [`timeline-selected-track`](../../assets/ui/timeline-selected-track.html) |

## API

```rust
use ui_storybook::components::editor::{TimelineSkeleton, TimelineView};
use ui_storybook::fixtures::editor::sample_timeline_skeleton;

view! { <TimelineSkeleton view=sample_timeline_skeleton() /> }
```

```admonish important title="Skeleton, not editing"
This is presentational scaffolding only. Timeline editing (clip
trimming, keyframe drag, mask scrubbing) is `DopeSheet`'s job and
lives in `wisp` plus the future editing controller. The skeleton
just shows row labels + placeholders the user can recognize.
```
