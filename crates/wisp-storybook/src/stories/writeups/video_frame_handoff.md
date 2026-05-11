A synthetic `decode::VideoFrame` (128×72 BGRA) is uploaded to a wisp
`VideoTexture` and drawn through the standard `Sprite` pipeline. The
seam is exactly `frame.bgra` → `VideoTexture::upload_bgra` — wisp
doesn't know whether the bytes came from a GStreamer pipe, a
ScreenCaptureKit callback, or a synthetic gradient.

The gradient + horizontal stripe pattern is deterministic per
`(width, height)`, so the quadrant fingerprint snapshot detects any
upload-orientation, format-byte-order, or colorspace regressions —
not just the existence of pixels.
