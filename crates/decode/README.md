# `decode` — video decode → BGRA frames

> Codec-agnostic seam between video files and `wisp`. Defines the
> `VideoStream` trait and ships two implementations: a synthetic
> `MockVideoStream` (no system deps, deterministic) and a
> `GstreamerPipeStream` that spawns `gst-launch-1.0` as a subprocess
> and reads BGRA frames off stdout.

## What it does

`decode` is a thin trait + two implementations. Consumers (`playback`,
`preview`, `screen-app`) take a `Box<dyn VideoStream>` and don't care
which backend produced the frames. Today the only real backend is
GStreamer via CLI subprocess; a future `gstreamer-rs` Rust-binding
backend is a one-line swap at the call site.

## Where it fits

```mermaid
flowchart LR
    classDef ours fill:#14532d,stroke:#16a34a,color:#bbf7d0
    classDef other fill:#374151,stroke:#9ca3af,color:#f3f4f6

    File["MP4 / WebM / ...<br/>(any container GStreamer<br/>can handle)"]:::other
    Decode["<b>decode</b><br/>VideoStream trait<br/>+ GstreamerPipeStream<br/>+ MockVideoStream"]:::ours
    Player["playback::Player"]:::other
    Wisp["wisp::VideoTexture"]:::other

    File --> Decode
    Decode -->|next_frame| Player
    Player -->|upload BGRA| Wisp
```

## Quickstart

```rust
use decode::{VideoStream, gstreamer_pipe::GstreamerPipeStream};

let mut stream = GstreamerPipeStream::open("video.mp4")?;
let (w, h) = (stream.width(), stream.height());
while let Some(frame) = stream.next_frame()? {
    // frame.bytes is BGRA, length = w * h * 4.
    // frame.pts is presentation timestamp.
    upload_to_gpu(&frame.bytes, w, h);
}
```

> [!IMPORTANT]
> GStreamer must be on `$PATH`. macOS: `brew install gstreamer`.
> Linux: `apt install gstreamer1.0-tools gstreamer1.0-plugins-base
> gstreamer1.0-plugins-good gstreamer1.0-libav`
> (`-libav` is required for H.264). Windows: not installed by default
> in CI — see
> [GStreamer integration choice](https://eng-manager-xyz.github.io/screen/media/architecture.html).

## Public API at a glance

| Item | Purpose |
|---|---|
| `VideoStream` trait | `next_frame() -> Result<Option<VideoFrame>>` + dimensions |
| `VideoFrame` | `{ bytes: Vec<u8>, pts: MediaTime }` |
| `MockVideoStream` | Synthetic gradients, no system deps |
| `gstreamer_pipe::GstreamerPipeStream` | Real decode via `gst-launch-1.0` subprocess |
| `gstreamer_pipe::gstreamer_available()` | Runtime probe — true iff the binary is on PATH and responds to `--version` |

Full rustdoc: [`api/decode/`](https://eng-manager-xyz.github.io/screen/api/decode/index.html).

## Runbook

### Build + test

```bash
cargo nextest run -p decode
cargo test -p decode --doc
cargo clippy -p decode --all-targets --all-features -- -D warnings
```

### Try it via consumers

`decode` is pure library — no binaries, no examples. Drive it through
the consumers:

```bash
cargo run -p playback --example play_file     # decode → wisp render → PNGs
cargo run -p preview                          # decode → winit window → playback
```

### Common tasks

**Add a new decode backend.** Implement `VideoStream` in a new module,
add a constructor (e.g. `MyBackend::open(path)`), wire it where the
consumer constructs the stream. The trait is intentionally minimal:
`next_frame()` + dimensions.

**Probe GStreamer at runtime.** Call
`gstreamer_pipe::gstreamer_available()`. Returns `false` if
`gst-launch-1.0` or `gst-discoverer-1.0` can't be spawned. Tests in
`decode`, `preview`, and `screen-app` integration suites use this to
skip cleanly when the binary is absent (Windows CI relies on this).

### Troubleshooting

> [!WARNING]
> **`fdsink fd=1` is the canonical stdout sink** for `gst-launch-1.0`.
> Don't try `filesink location=-` or shell redirection — they don't
> work inside `gst-launch-1.0`'s arg parser.

> [!WARNING]
> **Drop-kill the child.** `Drop` on `GstreamerPipeStream` calls
> `child.kill()` + `child.wait()`. Without the explicit kill,
> `gst-launch-1.0` keeps decoding into a dropped pipe and burns CPU.

> [!NOTE]
> **CI mystery:** on GitHub Ubuntu runners we've observed
> `gstreamer1.0-tools` apt-installing successfully, but later nextest
> processes failing with `ENOENT` on `gst-launch-1.0` spawn. Root
> cause unclear (possibly nextest process-isolation). Workaround:
> every gstreamer-using test checks `gstreamer_available()` and skips
> cleanly. Errors from `decode::gstreamer_pipe::Error::Spawn` include
> a `PATH` snapshot for diagnosis.

## Deep dive

- **[Decode book chapter](https://eng-manager-xyz.github.io/screen/decode/overview.html)**
- **[GStreamer integration choice — CLI pipe](https://eng-manager-xyz.github.io/screen/media/architecture.html)**
- **[CLAUDE.md](../../CLAUDE.md)** — "GStreamer / external CLI integration".

## License

MIT.
