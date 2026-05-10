# Recorder mock — M0.21

![recorder mock](../../assets/wisp/example-recorder-mock.png)

The M0.21 proof point: the full layered scene tree from
`recorder-features-and-render-api.md` §4, rendered headless.

The compositor stack, back-to-front:

1. **Gradient background panel** — a `Graphics` linear gradient covering NDC.
2. **Recording quad** — placeholder for the screen capture, drawn as a
   rounded `Graphics` with stroke (the real M2 path swaps in a `Sprite`
   wrapping a `VideoTexture` fed by ScreenCaptureKit).
3. **Camera bubble** — rounded `Sprite` in a corner.
4. **Cursor sprite + click ripple** — a small textured `Sprite` plus a
   `Graphics` ellipse scaled-up beneath it.
5. **Keyboard chip** — rounded `Graphics` with a `Text` label inside.
6. **Caption text** — `Text` under the recording quad.

Render output: `target/recorder_mock.png`. Stats reported on stdout:
`draw_calls=5, sprites=3, graphics=4, glyphs=43, meshes=0`. Every
primitive type wisp ships is exercised in one frame — if `recorder_mock`
runs cleanly, the public API surface is sufficient for the recorder's
editor preview.

```bash
cargo run -p wisp --example recorder_mock
```

The mock uses synthetic textures (a checker pattern for the recording
quad's stand-in, a simple sprite for the cursor). M2+ replaces these
with real capture frames; the scene-graph shape stays the same.

[`Stage`](../../api/wisp/struct.Stage.html) ·
[`Sprite`](../../api/wisp/struct.Sprite.html) ·
[`Graphics`](../../api/wisp/struct.Graphics.html) ·
[`Text`](../../api/wisp/struct.Text.html) ·
[Recorder feature inventory](../../../recorder-features-and-render-api.md) (offsite — workspace docs)
