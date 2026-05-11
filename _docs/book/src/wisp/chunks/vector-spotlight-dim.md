# Vector spotlight + inverse-dim

[Linear: AUT-59](https://linear.app/harwood/issue/AUT-59)

<table>
<tr>
<td valign="top" width="50%">

![](../../assets/wisp/spotlight.png)

*`apply_spotlight_vector` — focus a region while dimming the rest.*

</td>
<td valign="top" width="50%">

![](../../assets/wisp/dim-outside.png)

*`apply_dim_outside_vector` — same shape, inverse coverage:
attenuate everything outside the focus path.*

</td>
</tr>
</table>

`apply_spotlight_vector` (M-VEC.6) and `apply_dim_outside_vector`
(M-VEC.7, this chunk) are the vector-driven entry points for guiding
viewer attention. New in this chunk: `apply_dim_outside_vector` —
the path-accepting companion to `apply_dim_outside_data`.

```rust
use glam::Vec2;
use wisp::{DimStrength, Vector, VectorShape};

let diamond = Vector::new(VectorShape::path(vec![
    Vec2::new( 0.0,  0.6),
    Vec2::new( 0.6,  0.0),
    Vec2::new( 0.0, -0.6),
    Vec2::new(-0.6,  0.0),
]));
renderer.apply_dim_outside_vector(
    &app,
    &diamond,
    DimStrength::Heavy,
    &base,
    &output,
);
```

| Method | Shape source | Strength source |
|---|---|---|
| `apply_spotlight(MaskShape, Color, ...)` | analytic SDF | raw `Color` alpha |
| `apply_spotlight_vector(Vector, Color, ...)` | any vector (incl. paths) | raw `Color` alpha |
| `apply_dim_outside_data(DimOutside, ...)` | analytic SDF | `DimStrength` preset |
| `apply_dim_outside_vector(Vector, DimStrength, ...)` *(new)* | any vector (incl. paths) | `DimStrength` preset |

## Tests

`crates/wisp/tests/dim_outside_vector.rs`:
- `vector_dim_outside_matches_data_route_for_analytic_shape` —
  byte-equivalence with the existing `apply_dim_outside_data` path.
- `vector_dim_outside_path_dims_around_polygon` — diamond polygon
  preserves base inside, dims red to mid-range with `DimStrength::Medium`.

## Done when

- [x] Rect spotlight / inverse dim works (M-MASK.6/.7).
- [x] Rounded rect spotlight / inverse dim works.
- [x] Outside dim opacity configurable (`DimStrength`).
- [x] Inside region preserved (existing tests).
- [x] Path-accepting variant ships (`apply_dim_outside_vector`).
- [x] `just gate` green.
