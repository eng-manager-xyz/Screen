Text rendered to a `RenderTexture`, then run through
`DropShadowFilter` twice — once with a small `(dx, dy)` offset and
dark color (drop shadow), once with `(0, 0)` and a bright color
(glow). Two filter passes, two output sprites, one render pass to
compose them onto the paper-white backdrop.

No new wisp code — the existing M-FILTER drop-shadow pipeline accepts
any source RT. Text is just another RT.
