# Vector primitives — M-VEC.2 / AUT-54

`Vector::add_to_stage(&mut stage, parent)` (and the underlying
`to_graphics()`) make a `Vector` primitive renderable. Analytic
shapes (rect / rounded-rect / circle / ellipse) convert directly
into a `Graphics` node that the existing graphics pipeline draws.
Paths return `None` — visible path rendering lands in M-VEC.10.

The story shows five tiles end-to-end:

1. Rect with solid fill.
2. Rounded rect with solid fill.
3. Circle with linear-gradient fill.
4. Ellipse with fill + stroke.
5. Rounded rect with `with_opacity(0.4)` — opacity multiplies
   directly into the fill alpha at conversion time.

Transforms are honored via `Vector::with_transform(...)`, which
sets the produced `Graphics`'s container transform.
