//! `ui-storybook` — Leptos UI component gallery, modelled on rust-ui's
//! shadcn-style copy-paste components but living inside our workspace so we can
//! put them under the same gate / story / test discipline as wisp.
//!
//! # Layers
//!
//! - [`components`] — the actual Leptos `#[component]`s (`Button`, `DopeSheet`, …).
//! - [`stories`] — a flat registry that pairs each component with one or more
//!   demo views (the same shape as wisp-storybook's `stories::all_stories`).
//!
//! # Testing
//!
//! Components are exercised via SSR (`leptos::prelude::ssr::render_to_string`)
//! in `tests/snapshots.rs`. Each story snapshots its rendered HTML through
//! `insta` so any unintended structural change (class swaps, missing children,
//! attribute drift) trips the gate before it can ship.
//!
//! Browser viewing comes via `trunk serve` once a `csr` entry-point is wired
//! up — that's deliberately deferred so SSR + snapshots can land first.

pub mod components;
pub mod stories;
