# Perspective rotation — M0.19

<video controls autoplay muted loop playsinline width="320" src="../../assets/wisp/mesh-perspective.mp4">
  Your browser doesn't support inline video; see
  <a href="../../assets/wisp/mesh-perspective.mp4">mesh-perspective.mp4</a>.
</video>

A textured quad rotating around the Y axis with perspective foreshortening —
the M0.19 `Mesh` node + custom WGSL shader. The animation above is the
storybook's `tick` running for 3 seconds at 30 fps, captured to MP4 via
`just snapshots-wisp-animated` (gstreamer-backed).

The shader rotates the quad's vertex positions in 3D (`x` and `z` change
with `cos`/`sin`, `y` stays fixed), then projects with
`1 / (1 + z * persp_strength)`. At full edge-on the quad disappears (the
projection collapses); at face-on it looks like a normal sprite. Tunable
strength (`0.0` = orthographic, `1.0` = aggressive foreshortening).

For the recorder, this gives the camera-bubble "tilt-on-focus" treatment
and any time the recording quad needs to feel like a card flipping in space
rather than a flat sprite.

`Mesh` nodes still batch by texture — multiple meshes sharing one texture
render in a single draw call (verified by the
`meshes_sharing_texture_batch_into_one_draw_call` test).

---

[`Mesh` API](../../api/wisp/scene/struct.Mesh.html) · [Stories index](../stories.md)
