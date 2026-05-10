# Path stroke + arrows — M-VEC.10 / AUT-62

`PathBuilder` + adaptive Bezier flattening. Three demonstrations:

1. `Callout::arrow_to(from, to, ...)` — straight arrow with auto-
   sized arrowhead.
2. Quadratic Bezier — `PathBuilder::quad_to`, flattened with
   `tolerance = 0.005`.
3. Polyline freehand — `move_to` + a chain of `line_to`.

All three call `Path::stroke_to_graphics(width, color, tolerance)`
and add the resulting `Graphics` node to the stage. Per-segment
joins are butt-style for V1; mitered joins are a future
enhancement.
