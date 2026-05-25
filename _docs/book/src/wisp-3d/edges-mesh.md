# `EdgesMesh` + wireframe pipeline

Sharp-edge derivation + 1px hairline rendering. Mirrors `THREE.EdgesGeometry(geometry, 8°)`.

## Derivation

For every triangle edge:
1. Bucket on a 1e-4-quantised endpoint pair (so face-duplicated meshes — the pyramid's "apex appears 4 times" layout — still share edges).
2. Count owning triangles.
3. **Boundary edge** (1 triangle): always emit.
4. **Interior edge** (2 triangles): emit iff the angle between their face normals exceeds `angle_threshold_deg`.
5. **Non-manifold edge** (3+ triangles): always emit (to surface the mesh bug).

The pyramid at 8° produces 8 edges: 4 apex-to-base + 4 base perimeter. The internal diagonal of the square base is coplanar (180° dihedral) so it doesn't make the cut.

## Pipeline state

`WireframePipeline` uses `PrimitiveTopology::LineList` + carefully tuned depth state:

| Field | Value | Why |
|---|---|---|
| `depth_compare` | `LessEqual` | edges coincident with the mesh draw on top instead of z-fighting away |
| `depth_write_enabled` | `false` | wireframe doesn't occlude anything behind |
| `bias.constant` / `slope_scale` | `-1` / `-1.0` | push edges toward the viewer to break ties |
| `cull_mode` | `None` | line segments are 1D, no front/back |

```admonish bug title="1px hairlines only"
`PrimitiveTopology::LineList` produces 1-device-pixel-wide lines on every wgpu backend. Wider lines need screen-space-expanded quads (a follow-up — W3D.5.1). For the 404 page this is fine: the THREE version is also 1px (`LineBasicMaterial({ linewidth: 1 })`).
```
