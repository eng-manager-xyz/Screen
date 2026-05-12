# Recording card + library grid

[Linear: AUT-135](https://linear.app/harwood/issue/AUT-135)

The library's card primitive and the grid that lays cards out under
a filter + sort + layout toolbar. Card lifecycle covers Ready,
Processing (with percent overlay), and Failed.

<iframe src="../../assets/ui/library-grid-default.html" width="900" height="540" frameborder="0"></iframe>

## Card states

| State | Story |
| --- | --- |
| Ready | [`recording-card-ready`](../../assets/ui/recording-card-ready.html) |
| Processing | [`recording-card-processing`](../../assets/ui/recording-card-processing.html) |
| Missing thumbnail | [`recording-card-empty-thumbnail`](../../assets/ui/recording-card-empty-thumbnail.html) |

## Grid states

| State | Story |
| --- | --- |
| Default | [`library-grid-default`](../../assets/ui/library-grid-default.html) |
| Empty | [`library-grid-empty`](../../assets/ui/library-grid-empty.html) |
| List mode | [`library-grid-list-mode`](../../assets/ui/library-grid-list-mode.html) |

## API

```rust
use ui_storybook::components::library::{
    LibraryGrid, RecordingCard, RecordingCardState,
};
use ui_storybook::fixtures::library::{sample_library_grid, sample_recording_cards};

view! { <LibraryGrid view=sample_library_grid() /> }
```

```admonish important title="ThumbnailView is a CSS gradient"
Card thumbnails are CSS `background:` values, not real video frames.
That keeps the SSR snapshot deterministic across machines and
avoids dragging the renderer into the library surface. App-side
state will swap the gradient for a real
`url(file://...)` once the encoder writes a poster frame.
```
