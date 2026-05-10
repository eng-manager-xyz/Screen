# Dynamic mask textures — M-DYN.1 / AUT-43

`Renderer::generate_mask_texture(shape, w, h)` and the path-variant
`Renderer::generate_path_mask_texture(points, w, h)` produce
single-purpose coverage `RenderTexture`s. Output stores `(m, m, m, m)`
so consumers can sample as alpha (composition) or as RGB (display /
debug).

The story shows a contact sheet of all five shape sources rendered
as grayscale tiles:

1. Rect — sharp rectangle.
2. Rounded rect — same bounds, corner radius 0.25.
3. Circle — radius 0.85 from origin.
4. Ellipse — anisotropic (a=0.85, b=0.5).
5. Path — concave five-pointed star.

This primitive owns *only* coverage. Privacy blur, redaction, and
spotlight will compose with these textures via separate paths
(M-DYN.3+, M-VEC.4+). The next chunk (`AUT-44 / M-DYN.2`) layers a
cache on top so identical masks across frames don't regenerate.
