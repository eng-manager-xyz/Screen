```admonish important title="Architectural boundary"
**`wisp` does not depend on `media`, `decode`, `playback`, `capture`,
or any application crate.** The dependency arrows go one way:

- `media` / `decode` / `playback` produce data (BGRA frames, audio
  histograms, geometry) and hand it to `wisp` via standalone types
  it already owns (`VideoTexture`, `Sprite`, `Graphics`).
- `wisp` provides the scene graph; everything else composes against it.

Any change that makes wisp pull from a higher-level crate breaks the
ability to publish wisp to crates.io as a standalone renderer. See
`_docs/wisp-book/src/intro.md` for the publishable-crate contract.
```
