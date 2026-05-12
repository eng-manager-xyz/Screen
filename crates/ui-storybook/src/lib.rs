//! `ui-storybook` — Leptos UI component gallery, modelled on rust-ui's
//! shadcn-style copy-paste components but living inside our workspace so we can
//! put them under the same gate / story / test discipline as wisp.
//!
//! # Layers
//!
//! - [`components`] — the actual Leptos `#[component]`s (`Button`,
//!   `DopeSheet`, …), organised into product-surface subgroups
//!   (`primitives`, `shell`, `menus`, `recorder`, `library`, `editor`,
//!   `cursor`).
//! - [`fixtures`] — owned mock data structs (devices, workspaces,
//!   recordings, timeline tracks, …) reused across stories so component
//!   stories never hand-roll inline mocks.
//! - [`stories`] — the gallery registry. Every component re-exported from
//!   `components` must have at least one story; `stories::all_stories()`
//!   is the stable list consumed by `tests/snapshots.rs` and
//!   `ui-export-stories`.
//!
//! # Testing
//!
//! Components are exercised via SSR (`leptos::IntoView::into_view().to_html()`)
//! in `tests/snapshots.rs`. Each story snapshots its rendered HTML through
//! `insta` so any unintended structural change (class swaps, missing children,
//! attribute drift) trips the gate before it can ship.
//!
//! Browser viewing comes via `trunk serve` once a `csr` entry-point is wired
//! up — that's deliberately deferred so SSR + snapshots can land first.

pub mod components;
pub mod exporter;
pub mod fixtures;
pub mod stories;
