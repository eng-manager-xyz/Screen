A solid dot on the left; the same dot smeared along a `(900, 600)` velocity vector on the right.

`MotionBlurFilter` reuses the separable Gaussian shader from `BlurFilter` but swaps the axis-aligned `(1,0)` / `(0,1)` directions for the unit-velocity vector. Kernel size scales with `velocity.length() / peak_velocity_pps` clamped at `max_kernel_px` — constants `1400` / `14` lifted from OpenScreen's `zoomTransform.ts`.

For the recorder this is the foreshadowing motion blur during zoom transitions and panning. When the camera (the recording viewport) is in motion, every frame's source content gets smeared along the velocity vector — the same trick Screen Studio uses to make zoom feel cinematic instead of jarring.
