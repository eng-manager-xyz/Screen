## Cursor overlay — ED.19

A raw screen recording bakes the OS cursor into the pixels — tiny, jittery,
and gone the moment it stops moving. ED.19 replaces it with a *groomed*
cursor drawn at compose time from the recorded **cursor track**: a scaled,
dark-outlined white pointer (so it reads on any backdrop) with an expanding
click ripple that marks where the user acted.

The overlay is a single `Graphics` node over the framed screen:

- **Pointer** — two stacked CCW triangles (`draw_polygon`), a dark one behind
  a white one, sized by the project's `size_pct`.
- **Ripples** — fading, expanding discs (`draw_ellipse`) at each recent
  click, their radius growing and alpha decaying across a ~0.4 s window
  (`edit::telemetry::ripples_at`).

The load-bearing decision: the cursor **rides the screen transform**. The
captured position is normalized to the source frame, so it's mapped through
the *same* crop / zoom / padding transform as the screen sprite — the pointer
stays glued to the exact pixel it was over, magnifying with the auto-zoom
punch-in rather than drifting off the button it clicked. Smoothing is a pure
EMA over the track (`edit::telemetry::cursor_at`), taming jitter the way a
fluid head tames handheld.
