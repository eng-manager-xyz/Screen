# `playback` — player state machine + frame pump

> The middle layer between `decode` and `wisp`. `Player` owns a boxed
> `VideoStream` and a `VideoTexture`, pumps decoded frames into the
> GPU at the source's frame rate, and exposes `Empty` / `Paused` /
> `Playing` / `Ended` as a tiny state machine.

## What it does

`Player` is one tick per render frame: the host (Tauri shell, winit
preview) calls `player.tick(dt)`; if the player is `Playing` and
enough wall-clock time has elapsed, the next decoded frame uploads to
the `VideoTexture`. The shell then renders the wisp `Stage` (which has
a `Sprite` referencing that texture) as usual.

This decouples the frame pump from the renderer rhythm: the renderer
runs at vsync; the player runs at source frame rate.

## Where it fits

```mermaid
sequenceDiagram
    autonumber
    participant Shell as Shell<br/>(Tauri / winit)
    participant Player
    participant Stream as Box&lt;dyn VideoStream&gt;<br/>(decode)
    participant Texture as VideoTexture<br/>(wisp)
    participant Renderer as wisp::Renderer

    loop once per render frame
        Shell ->> Player: tick(dt)
        Note over Player: if Playing,<br/>elapsed += dt
        alt next frame is due
            Player ->> Stream: next_frame()
            Stream -->> Player: VideoFrame { bytes, pts }
            Player ->> Texture: upload_bgra(bytes)
        end
        Shell ->> Renderer: render_stage(view, &stage)
    end
```

## Quickstart

```rust
use playback::Player;
use decode::gstreamer_pipe::GstreamerPipeStream;

let mut player = Player::new(&app);
player.open(Box::new(GstreamerPipeStream::open("video.mp4")?))?;
player.play();

// Per frame, in the host's render loop:
player.tick(dt);          // pumps decoded frames into player's VideoTexture
let stage = build_scene(&player);
renderer.render_stage(&app, view, clear, &stage);
```

The full headless example is `cargo run -p playback --example play_file`.

## Public API at a glance

| Item | Purpose |
|---|---|
| `Player::new(app)` | Construct with an empty `VideoTexture` |
| `Player::open(stream)` | Bind a `VideoStream`, transition to `Paused` |
| `Player::play()` / `Player::pause()` | Transport controls |
| `Player::tick(dt)` | Frame pump (call once per render frame) |
| `Player::status()` | `{ state, elapsed, duration, last_pts }` |
| `Player::texture()` | The `VideoTexture` for binding to a `Sprite` |
| `PlayerState::{Empty, Paused, Playing, Ended}` | State machine |

Full rustdoc: [`api/playback/`](https://eng-manager-xyz.github.io/Screen/api/playback/index.html).

## Hero output

![play_file frame 3](../../_docs/book/src/assets/playback/playfile_03.png)

Frame 3 of 7 — the `play_file` example decodes the bundled MP4 fixture
end-to-end through `decode` → `Player` → `wisp::VideoTexture` →
`Renderer` → PNG. The motion is the gradient phase-shifting frame to
frame; proves the full pipeline.

## Runbook

### Build + test

```bash
cargo nextest run -p playback
cargo test -p playback --doc
cargo clippy -p playback --all-targets --all-features -- -D warnings
```

### Run the headless example

```bash
cargo run -p playback --example play_file
# Output: _docs/book/src/assets/playback/playfile_NN.png
```

Decodes `crates/decode/tests/fixtures/sample.mp4` (11 KB committed
fixture), uploads each frame to `VideoTexture`, renders through wisp,
reads pixels back, writes PNGs.

### Common tasks

**Wire `Player` into a new host.** Implement the tick loop:

```rust
loop {
    player.tick(dt);             // pump decoded frames
    renderer.render_stage(...);  // your render pass
}
```

The host owns the tick cadence (winit's `RedrawRequested`, Tauri's
33 ms thread, etc.). `Player` does the math.

**Drive transport from IPC.** See `screen-app`'s `commands.rs` for
the canonical Tauri IPC pattern: `player_open` / `player_play` /
`player_pause` / `player_status` commands +
`player-status` event emitted on state transitions.

### Troubleshooting

> [!NOTE]
> **First tick catches up multiple frames.** When `Player::play()`
> fires at `t=0`, the first `tick(dt)` might emit two frames if the
> source frame rate is faster than the tick rate. This is intentional
> — keeps wall-clock alignment. The `play_file` example demonstrates
> this on frame 0.

> [!WARNING]
> **Paused players don't advance `elapsed`.** Tick is a no-op when
> `state != Playing`. If you see `elapsed == 0` after a 1 s tick,
> check that `play()` was called.

## Deep dive

- **[Playback book chapter](https://eng-manager-xyz.github.io/Screen/playback/overview.html)**
- **[Real MP4 → wisp playback](https://eng-manager-xyz.github.io/Screen/playback/play-file.html)**
- **[`decode`](../decode/README.md)** — the `VideoStream` source.
- **[`wisp`](../wisp/README.md)** — the `VideoTexture` sink.

## License

MIT.
