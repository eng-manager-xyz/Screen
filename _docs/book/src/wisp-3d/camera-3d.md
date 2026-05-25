# `Camera3D`

Perspective camera mirroring `THREE.PerspectiveCamera`'s call shape: FOV (degrees) + aspect + near + far + position/target/up.

## Conventions

- Right-handed. `glam::Mat4::look_at_rh` for the view; `Mat4::perspective_rh` for the projection.
- wgpu NDC depth range is `[0, 1]` — we use `perspective_rh` (not `_rh_gl`, which gives OpenGL's `[-1, 1]` and wastes half the depth precision).

## GPU-side uniform

`ViewProj` is `#[repr(C, align(16))]` carrying `view + proj + view_proj + camera_pos`. 208 bytes; layout-tested.

```admonish important title="Match THREE's call shape"
`Camera3D::perspective(fov_deg, aspect, near, far)` is degree-in for a reason — the engmanager.xyz 404 page hardcodes `PerspectiveCamera(38, aspect, 0.1, 100)`. Constructor degree-in keeps the port mechanical.
```

## Resize

`update_aspect(width, height)` clamps `height >= 1` so a minimised window doesn't NaN the projection. Doesn't move the camera or change FOV — only the projection matrix shifts.

```rust,no_run
# use wisp_3d::Camera3D;
# use glam::Vec3;
let mut cam = Camera3D::perspective(38.0, 16.0 / 9.0, 0.1, 100.0);
cam.position = Vec3::new(0.0, 0.28, 6.2);  // 404 page values
// On window resize:
cam.update_aspect(1920, 1080);
// Per-frame upload:
let uniform = cam.view_proj_uniform();
```
