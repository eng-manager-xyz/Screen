A rounded rect rendered offscreen, then composited on top of its own blurred shadow.

DropShadowFilter does four passes inside a single Filter::render_pass call: it allocates two scratch RenderTextures, extracts the source alpha (offset and tinted) into scratch_a, runs separable Gaussian blur (h then v) using BlurFilter's pipeline, then composites the source over the blurred shadow with alpha-over math.

For the recorder this gives the cinematic recording-card look: a padded recording quad floating over a wallpaper background with a soft drop shadow underneath. The shadow color/alpha controls how grounded the recording feels; the offset controls perceived light direction.

Re-using BlurFilter's `run_blur_pass` between filters keeps the shader sharing tight — DropShadow doesn't duplicate Gaussian math, it just calls into it.
