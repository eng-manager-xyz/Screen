Three copies of the same source — identity, grayscale, and brightness ×1.4 — using `ColorMatrixFilter`.

The shader applies a 4×5 matrix (`out = M · [r, g, b, a, 1]`). Named constructors give common operations: `identity()`, `grayscale()` (Rec.709 luminance weights), `brightness(scale)`. Compose them by chaining filter applications, or build a custom matrix for tone curves, channel shuffles, sepia, etc.

For the recorder this becomes: per-clip color grading, accessibility filters (high contrast, deuteranopia simulation), and the building block for any "look" preset.
