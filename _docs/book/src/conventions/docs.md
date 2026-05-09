# Documentation gate

The workspace lints `missing_docs = "warn"`. `just docs` (in `just gate`)
builds rustdoc with that warning surfaced. `just docs-strict` flips it to
`-D warnings` plus `-D rustdoc::broken_intra_doc_links` for milestone close.

## Per-chunk requirements

Every chunk must update:

1. **Crate-level `//!` header** if the architecture changed (new module,
   new public surface, new feature flag).
2. **`///` doc on every new public item** — types, fields, variants, methods,
   functions. The `missing_docs` lint catches every miss.
3. **At least one `# Examples` doctest** on each new public function. Doctests
   run via `cargo test --doc` (in `just gate`'s `doctest`), so they double as
   anti-rot for the documentation.

## Recommended doc structure

```rust
//! Crate-level header — what this crate is for.
//!
//! # Overview
//! …
//!
//! # Quick start
//! ```rust
//! use thiscrate::Foo;
//! let foo = Foo::new();
//! foo.do_thing();
//! ```
//!
//! # Architecture
//! …
```

```rust
/// One-line summary.
///
/// Longer paragraph if needed.
///
/// # Examples
///
/// ```rust
/// # use thiscrate::Foo;
/// let foo = Foo::new();
/// assert_eq!(foo.value(), 0);
/// ```
pub fn new() -> Self { … }
```

## Tooling

| Command | What it does |
|---|---|
| `just docs` | `cargo doc --workspace --no-deps`. In `just gate`. |
| `just docs-strict` | Same, with `-D warnings`. Run before milestone close. |
| `just site` | Builds mdBook + rustdoc, writes `target/book/`. |
| `cargo test --doc` | Runs every `# Examples` block. In `just gate` via `doctest`. |
