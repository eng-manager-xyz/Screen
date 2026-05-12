# Review checklist

[Linear: AUT-143](https://linear.app/harwood/issue/AUT-143)

Use this list when reviewing a PR that touches
`crates/ui-storybook/src/components/`. Each line maps to a rule in
the [presentational contract](./presentational-contract.md).

## File scan

- [ ] No `use leptos::reactive::*;` outside a `cfg(test)` block.
- [ ] No `RwSignal::new`, `signal(`, `Effect::new`, `Effect::watch`,
      `Action::new` in `components/`.
- [ ] No `tauri::`, `invoke(`, `wasm_bindgen::start`,
      `web_sys::window().local_storage()`.
- [ ] No `set_interval`, `setTimeout`, `gloo_timers`.
- [ ] No `std::fs`, `std::process`, `tokio::spawn` inside components.
- [ ] No `lazy_static!`, `OnceCell`, `Lazy`-style globals.

## Props / API surface

- [ ] Every visual state has a named prop (`selected`, `open`,
      `disabled`, `loading`, etc.) — not a derived internal bool.
- [ ] Optional callbacks are typed `Option<Callback<T>>` with
      `#[prop(optional)]`. None is the SSR-stable default.
- [ ] Long view-model structs decompose at the top of the component
      so the `view!` body reads as flat HTML.
- [ ] No `Option<Option<T>>` from accidentally wrapping
      `Option<Children>` in `Some(...)`.

## Story coverage

- [ ] Every new variant has at least one story in
      `ui_storybook::stories::all_stories()`.
- [ ] Stories sweep at minimum: default + active/open + disabled +
      empty/overflow.
- [ ] Story id is kebab-case and matches the asset HTML filename.
- [ ] If the new component takes a `Children` slot, at least one
      story exercises it.

## mdBook chapter

- [ ] New chapter under `_docs/book/src/ui/chunks/<id>.md`.
- [ ] Iframe embed of the default story near the top.
- [ ] States table.
- [ ] API code block.
- [ ] At least one `admonish important` for the non-obvious rule.
- [ ] Mermaid (no ASCII) if a diagram is needed.
- [ ] Listed in `_docs/book/src/SUMMARY.md`.

## Gate

- [ ] `just snapshots-ui` re-exports the HTML assets.
- [ ] `just gate` is green (fmt + check + clippy + nextest + doctest +
      docs + snapshots-check + mermaid-check).
- [ ] `PROGRESS.md` has a new entry with files / tests / verified
      lines.

## Wisp / canvas region only

- [ ] Component declares a `CanvasBackendView`-style enum.
- [ ] CSS fallback renders without any browser API.
- [ ] If a Wisp asset is referenced, the PNG is committed under
      `_docs/book/src/assets/ui/`.
- [ ] No direct `wgpu` import in the component.

## Common foot-guns

- [ ] `Show when=…` closure is `'static` — captures a bool, not a
      borrowed `String`.
- [ ] No `Some(ToChildren::to_children(...))` — pass bare for
      optional slots.
- [ ] Component file under 100 lines per fn (clippy
      `too_many_lines`); split the view into helpers.
- [ ] Pre-existing rustdoc intra-doc links don't reference
      `ComponentName::method` — Leptos components are fns, not
      types.
