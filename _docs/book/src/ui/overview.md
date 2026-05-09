# `ui-storybook` — overview

The HTML/Leptos counterpart to `wisp-storybook`. Every shipped UI component
appears here, with its SSR HTML locked to an `insta` snapshot — same regression
discipline as wisp's quadrant fingerprints.

Rendered demos live under [`assets/ui/`](../assets/ui/). Each is a complete
standalone HTML file with the storybook stylesheet inlined; open it in a
browser tab for a live render.

Regenerate with `just snapshots-ui`.

## Visual baseline: rust-ui

The component set follows [rust-ui](https://www.rust-ui.com)'s shadcn-style
copy-paste convention but lives in this workspace so we can put it under the
same gate as the rest of the code.

Visual style: zinc dark palette, subtle 1px borders, 6/10px radius corners,
muted secondary text, accent for primary actions and the playhead.

## Index

- [Components](./components.md) — Button, Card.
- [Dope sheet](./dope-sheet.md) — net-new editor timeline.
