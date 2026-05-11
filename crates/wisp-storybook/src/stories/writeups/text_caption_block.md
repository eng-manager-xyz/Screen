Two `CaptionBlock`s — one single-line, one wrapped across multiple
lines. Both use the same width and padding; the block measures the
wrapped text and sizes its rounded background to match.

The block is pure composition — text texture on top of a `Graphics`
rounded rect. The wrapped text uses cosmic-text's normal layout via
`WispText::with_wrap`; the background grows as the text wraps.
