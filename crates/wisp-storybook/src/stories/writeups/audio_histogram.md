A synthetic sine wave quantized to an `AudioHistogram` (M-MEDIA.8),
laid out as bar rectangles via `media::waveform::mono_bars` (M-MEDIA.9),
and rendered through Wisp's `Graphics` pipeline as plain colored
rects.

`wisp` doesn't know about audio. `media` produces typed geometry
(`Vec<WaveformBarRect>`); `wisp` draws rectangles. The seam is the
function call between them — nothing else crosses.

The story uses a 440 Hz sine at amplitude 0.6, sampled at 48 kHz for
1 second, quantized at 50 ms (20 bars), rendered mirrored about
`baseline_y = 0` in NDC. Color is a soft amber.

For the recorder, this primitive becomes: timeline waveform display,
dope-sheet audio rows, recording-level meter, and the "is the mic
picking anything up?" indicator.
