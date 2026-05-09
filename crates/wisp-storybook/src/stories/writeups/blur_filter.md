Sharp source on the left; the same texture, blurred via a two-pass separable Gaussian, on the right.

The Filter trait is straightforward: `passes()` returns the number of render passes the filter needs (BlurFilter returns 2 — one horizontal, one vertical), and `render_pass(ctx, input, output, pass)` does the work for that pass. The Renderer's `apply_filter` orchestrator allocates a scratch RenderTexture for multi-pass filters and ping-pongs.

For the recorder this gives us:
- The mask/highlight tool's blurred sensitive regions.
- The glassmorphism background variant.
- A building block for DropShadow (M0.17 = alpha extract → BlurFilter → composite).

The 9-tap kernel uses a small fixed weight table tuned for visual blur quality at typical UI radii (1–8 texels). For larger blurs we'd ramp the kernel size or run multiple passes.
