# `playback` — overview

The middle layer between [`decode`](../decode/overview.md) and
[`wisp`](../wisp/overview.md). `Player` owns a boxed `VideoStream` and a
`VideoTexture`, and pumps decoded frames into the GPU at the source's
frame rate while the shell ticks it once per render frame.

## Contract

```text
Shell (Tauri / winit)
   │  player.tick(dt)             once per render frame
   ▼
Player                            owns:
   │  ├─ Box<dyn VideoStream>     ← decode crate
   │  └─ VideoTexture             ← wisp crate
   │
   │  per tick:
   │    1. if Playing, advance elapsed += dt
   │    2. while elapsed >= next_due:
   │         frame = stream.next_frame()
   │         texture.upload_bgra(&frame.bgra)
   │         next_due += 1.0 / stream.frame_rate()
   │
   ▼
GPU (VideoTexture is now current frame)
   │
   ▼
wisp Sprite::from_texture(player.texture()) → on screen
```

`tick` returns the number of frames it actually uploaded so the shell
can drive a redraw signal off it (no re-render needed when no new frame
is due).

## Transport

| State | What `tick` does |
|---|---|
| `Paused` (default) | nothing — `elapsed` stays put, texture serves the last uploaded frame |
| `Playing` | normal pump |
| `Ended` | nothing — same as `Paused` but the UI can swap "Pause" → "Replay" |

## Anti-regression contract (tests in `tests/timing.rs`)

- Paused player does not advance — `elapsed == 0` after a 1 s tick.
- First tick uploads the t=0 frame even with a 1 ms `dt` (no off-by-one
  causing the first frame to be skipped).
- 1 s of wallclock at 60 Hz render against a 30 fps source pulls
  ~30 frames (29..=31 inclusive — boundary tolerance is documented).
- Stream exhaustion transitions to `Ended` cleanly.
- Pause freezes both the state *and* the texture (next `tick` does not
  re-upload the held frame).
- `duration_hint` matches `frame_count / frame_rate`.

## Visual proof — 30 ticks of the timed_playback example

Run with:

```
cargo run -p playback --example timed_playback
```

Each tick where the player uploaded a frame is captured below — that's
the gradient phase advancing across 1 s of wallclock as the player
catches the timestamps that come due.

| Tick 00 | Tick 02 | Tick 04 | Tick 06 |
|---|---|---|---|
| ![](../assets/playback/tick_00.png) | ![](../assets/playback/tick_02.png) | ![](../assets/playback/tick_04.png) | ![](../assets/playback/tick_06.png) |

| Tick 11 | Tick 21 | Tick 31 | Tick 41 |
|---|---|---|---|
| ![](../assets/playback/tick_11.png) | ![](../assets/playback/tick_21.png) | ![](../assets/playback/tick_31.png) | ![](../assets/playback/tick_41.png) |

| Tick 49 | Tick 53 | Tick 57 | Tick 59 |
|---|---|---|---|
| ![](../assets/playback/tick_49.png) | ![](../assets/playback/tick_53.png) | ![](../assets/playback/tick_57.png) | ![](../assets/playback/tick_59.png) |

[Player API](../api/playback/struct.Player.html) ·
[`PlayState`](../api/playback/enum.PlayState.html)
