# Solid redaction — M-MASK.5 / AUT-23

`Renderer::apply_solid_redaction(shape, color, base, output)` fills a
shape with an opaque color over the base. This is the *trust* mode —
unlike privacy blur (which may leave faint structure visible), solid
redaction guarantees no information leaks through.

The story shows two variants side-by-side over the same gradient +
grid backdrop:

- **Left** — sharp rectangle redaction.
- **Right** — rounded rectangle redaction (matches the cinematic
  rounded-corner aesthetic of modern app surfaces).

Both use the same primitive — only the `MaskShape` differs. Future
shape variants (circle, ellipse, freehand path) all plug in here for
free.

## Why solid over blur

Privacy blur is polish: text becomes unreadable but shape and motion
hint through. That's fine for "I'd rather not show this email" but it
is *not* safe enough for "this is an API key." For secrets — passwords,
auth tokens, customer IDs — a partial reconstruction is still a leak.
Solid redaction is the only treatment we can recommend in that case.

UI copy (when the editor surface lands) should communicate this
distinction — solid is the safe default for highly sensitive content,
blur is the polished default for visual privacy.
