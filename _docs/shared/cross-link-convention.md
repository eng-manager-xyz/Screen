```admonish info title="Cross-book link convention"
This is one of **two** sibling mdBooks deployed to the same GitHub
Pages site:

- **Project book** — `/Screen/` (recorder + capture + encoder + Tauri shell).
- **Wisp book** — `/Screen/wisp/` (renderer-only reference; publishable to crates.io independently).

Cross-references in either book go through the
`mdbook-preprocessor-cross` preprocessor so URLs adapt per book:

- `\{\{wisp-link path/to/chunk\}\}` — emits a relative URL inside
  the wisp book, an absolute `/Screen/wisp/path/to/chunk.html` URL
  from the project book.
- `\{\{shared path/to/fragment.md\}\}` — inlines a markdown
  fragment from `_docs/shared/` (this snippet you're reading is
  one).

Plain markdown links from the wisp book back to the project use
absolute URLs (`/Screen/...`) since the inverse direction is
single-target.
```
