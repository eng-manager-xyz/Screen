---
name: leptos-migration
description: Maps every prior Leptos API (0.1 → 0.7) to the current 0.8 surface, with "strive to use" callouts for the modern idioms. Invoke when upgrading old Leptos code, when an example/StackOverflow snippet uses pre-0.8 patterns, or when picking the right primitive for new code.
---

# Leptos migration — every version → 0.8

This project pins **`leptos = "0.8"`**. Old StackOverflow answers, README examples, and AI-generated snippets reach for older idioms. This skill is the translation table.

Each section starts with a comparison table, then unpacks the "strive to use" 0.8 idiom underneath. When in doubt, **prefer the 0.8 column** — even if the older form still compiles.

---

## 0.0 — current pinned version + invariants

- `leptos = { version = "0.8", default-features = false, features = [...] }` — every workspace consumer pins to `"0.8"`.
- This project's two consumers:
  - `crates/ui-storybook` — SSR + storybook gallery (`"ssr"` default; `"csr"` for Trunk).
  - `crates/app-ui` — Tauri webview, CSR-only (`"csr"`).
- `wisp` is renderer-only, **no leptos**. Leptos lives entirely in the UI layer.

```admonish important
Components in `ui-storybook/components/**` are **presentational and stateless** — props in, callbacks out. **No `signal()`, `RwSignal::new()`, `Effect::new()`, `Action::new()`, or anything reactive inside a component module.** App-side state lives in `crates/app-ui`. See `_docs/book/src/ui/presentational-contract.md`.
```

---

## 1. Signals — read + write

| Era | API | Status in 0.8 |
| --- | --- | --- |
| 0.3 – 0.5 | `let (r, w) = create_signal(cx, 0);` | **Removed** — `cx` parameter no longer exists |
| 0.6 | `let (r, w) = create_signal(0);` | **Removed** — function deprecated |
| 0.7 – 0.8 | `let (r, w) = signal(0);` ← strive | ✅ standard |
| 0.7 – 0.8 | `let s: RwSignal<i32> = RwSignal::new(0);` ← strive when read+write share a name | ✅ standard |

```rust
// 0.8 idioms — pick whichever names read more cleanly at the call site.
use leptos::prelude::*;

// Split read/write — best when the read half is passed deeper than the write half.
let (count, set_count) = signal(0);
set_count.set(count.get() + 1);

// Combined read+write — best when the same scope reads and writes.
let count = RwSignal::new(0);
count.update(|n| *n += 1);
```

```admonish tip title="When to reach for which"
- `signal(...)` returns `(ReadSignal, WriteSignal)`. Pass the read half down to children that only display, and keep the writer at the top. **This is the default for new code.**
- `RwSignal::new(...)` collapses both halves into one. Use when the writer + reader live in the same component and you want one variable name. Convertible: `(rw.read_only(), rw.write_only())`.
```

### Type-annotated signal — turbofish vs let-typed

```rust
// 0.7+ pattern (works in 0.8): turbofish on the signal constructor.
let (loaded, set_loaded) = signal::<Option<String>>(None);
```

### `Memo<T>` — derived signal

| Era | API |
| --- | --- |
| ≤ 0.6 | `create_memo(cx, move || ...)` |
| 0.7 – 0.8 ← strive | `Memo::new(move |_| ...)` |

Note the **closure now takes the previous value as `Option<&T>`** so you can short-circuit equality checks.

```rust
let evens = Memo::new(move |_prev| count.get() % 2 == 0);
```

---

## 2. Effects

| Era | API |
| --- | --- |
| ≤ 0.6 | `create_effect(cx, move |_| ...)` |
| 0.7 – 0.8 ← strive | `Effect::new(move |_| ...)` |
| 0.8 only ← strive | `Effect::watch(deps_fn, handler_fn, immediate: bool)` |

```rust
use leptos::prelude::*;

// Generic reactive effect — re-runs when any tracked signal inside changes.
Effect::new(move |_prev| {
    leptos::logging::log!("count is {}", count.get());
});

// 0.8 strive: explicit watch — separates dependencies from work.
Effect::watch(
    move || count.get(),                 // dependency_fn — return is hashed for change detection
    move |new, _prev, _initial| {        // handler_fn — runs only when dep changes
        leptos::logging::log!("count is now {new}");
    },
    false,                               // immediate: run handler once on setup?
);
```

```admonish warning title="Effects do not belong in presentational components"
This project's UI components stay state-free. `Effect::*` lives in `app-ui` (where signals/IPC/timers are owned). The UI-23 grep guardrail flags any `Effect::new` inside `crates/ui-storybook/src/components/`.
```

---

## 3. Resources + async data

| Era | Pattern |
| --- | --- |
| ≤ 0.6 | `create_resource(cx, source, fetcher)` |
| 0.7 | `Resource::new(source, fetcher)` — required `Send + Sync` fetchers |
| 0.8 ← strive | `Resource::new(source, fetcher)` — same shape, slightly different bounds |
| 0.8 ← strive | `LocalResource::new(async fetcher)` — non-Send futures (e.g. wasm, `web-sys`) |

**0.8 breaking change**: `LocalResource` no longer exposes a `SendWrapper` in its return type.

```rust
// 0.7 — required .as_deref() / explicit deref past the SendWrapper:
let data = resource.get().as_deref().map(|d: &MyType| d.field.clone());

// 0.8 — drop the .as_deref(); the resource's get() returns the inner type directly.
let data = resource.get().map(|d: MyType| d.field.clone());
```

```admonish important
When you see `.as_deref()` on a `LocalResource` value in pre-0.8 code, **remove it** on upgrade. Test under SSR + CSR — error messages get more cryptic the longer you carry the wrapper.
```

`Suspend::new()` (0.7+) accepts any `IntoFuture` in 0.8 — you can pass `async { ... }` directly without manually boxing.

---

## 4. Actions

| Era | API |
| --- | --- |
| ≤ 0.6 | `create_action(cx, |input| async move { ... })` |
| 0.7 – 0.8 ← strive | `Action::new(\|input\| async move { ... })` |
| 0.7 – 0.8 ← strive | `Action::new_unsync(...)` — non-Send actions (wasm) |
| 0.8 only ← strive | `Action::new_local(...)` — thread-local actions, **with much-improved DX** |

```rust
let save_form = Action::new(|input: &MyFormData| {
    let payload = input.clone();
    async move { server_fn::save(payload).await }
});

save_form.dispatch(form_data);
let pending = save_form.pending();          // ReadSignal<bool>
let value   = save_form.value();            // RwSignal<Option<Result<...>>>
```

```admonish bug title="0.8 fix you actually feel"
Thread-local Actions had real bugs in 0.7 — leaks, missing re-renders, weird ownership semantics around `Action::new_local`. 0.8 fixed those plus a slew of `LocalResource` ownership issues. **If you have a 0.7 codebase that uses `Action::new_unsync` defensively, try `Action::new` first on 0.8 — many of those workarounds are no longer needed.**
```

---

## 5. `view!` macro + components

### Component definition

| Era | Definition shape |
| --- | --- |
| ≤ 0.5 | `fn component(cx: Scope, ...) -> impl IntoView` |
| 0.6 – 0.8 ← strive | `#[component]\nfn Component(...) -> impl IntoView` — **no `cx` parameter** |

```admonish warning title="0.5 → 0.6 was the great `cx`-removal"
The single biggest cosmetic break in Leptos history was 0.5 → 0.6 dropping the `Scope` parameter from every signal, effect, resource, component, and helper. If you see `cx: Scope` anywhere, it's pre-0.6 code. **Delete the parameter, delete every `cx,` argument inside.**
```

### Children slots — `Children` vs `ChildrenFn` vs `ChildrenFnMut`

| Era | Slot type | Use |
| --- | --- | --- |
| 0.7 – 0.8 ← strive | `children: Children` | One-shot rendering (FnOnce → AnyView). **Default for slots.** |
| 0.7 – 0.8 | `children: ChildrenFn` | Multi-render (Fn → AnyView). Use for slots inside loops / conditionals that re-render the body. |
| 0.7 – 0.8 | `children: ChildrenFnMut` | Stateful re-rendering. Rare. |
| 0.7 – 0.8 ← strive | `#[prop(optional)] inspector: Option<Children>` | Optional slot. |

```rust
#[component]
pub fn AppShell(
    rail: Children,
    main: Children,
    #[prop(optional)] inspector: Option<Children>,
) -> impl IntoView { /* ... */ }
```

To populate an `Option<Children>` from a story / parent:

```rust
view! {
    <AppShell
        rail=ToChildren::to_children(move || view! { <NavigationRail .. /> })
        main=ToChildren::to_children(move || view! { /* main pane */ })
        inspector=ToChildren::to_children(move || view! { /* inspector */ })
    />
}
```

```admonish bug title="Optional `Children` props take `Children`, NOT `Option<Children>`"
On 0.7+ the `#[prop(optional)]` macro internally wraps the value in `Option`. **Don't pass `Some(...)`** — pass the bare `ToChildren::to_children(...)` value (or omit the prop entirely to leave it `None`). We hit this in UI-02 when wiring the `AppShell` slots.
```

### Event handler syntax

| Era | Pattern |
| --- | --- |
| 0.5 | `on:click=move \|ev\| { ... }` — `ev` was an opaque type |
| 0.7 ← strive | `on:click=move \|ev: web_sys::MouseEvent\| { ... }` — concrete `web_sys` types |
| 0.8 ← strive | `on:input:target=move \|ev\| { ... }` — typed `ev.target().value()` directly via `:target` modifier |

The `:target` modifier (0.8) gives you an event with `ev.target()` already typed to the correct `web_sys` element — no `dyn_into::<HtmlInputElement>()` boilerplate.

### Conditional rendering — `<Show>`

```rust
// 0.7 + 0.8 unchanged shape:
<Show
    when=move || loaded.get().is_some()
    fallback=move || view! { <DropZone state=DropZoneState::Idle /> }
>
    { /* shown when when=true */ }
</Show>
```

```admonish important title="`when` must be `'static`"
`Show`'s `when` closure must be `'static`. If you capture a `String` from outside, **clone a `bool` instead** and re-derive the string inside the children. We documented this in CLAUDE.md under "Leptos `#[component]` specifics".
```

### `<ShowLet>` (0.8)

New in 0.8. Like `<Show>` but binds the truthy value:

```rust
<ShowLet
    when=move || loaded.get()
    let:path
>
    <PlayerView path=path />
</ShowLet>
```

### `<Either/>` + `Either!` macro (0.8)

`Either!` (0.8) makes multi-branch conditional rendering cleaner — replaces nested `<Show>` in many cases.

```rust
use leptos::either::Either;

let view = match recording_state {
    RecordingState::Idle      => Either::Left(view! { <RecordIdle /> }),
    RecordingState::Recording => Either::Right(view! { <RecordActive /> }),
};
```

---

## 6. SSR rendering — `RenderHtml::to_html()`

This is **the project's specific SSR entry point** (see `ui-storybook/src/stories/mod.rs::render`).

| Era | Pattern |
| --- | --- |
| 0.6 | `leptos::ssr::render_to_string(\|\| view! { ... })` |
| 0.7 – 0.8 ← strive | `view.into_view().to_html()` — `RenderHtml::to_html` brought into scope by `leptos::prelude::*` |

```rust
fn render<V: leptos::IntoView>(view: V) -> String {
    use leptos::prelude::*;
    view.into_view().to_html()
}
```

```admonish note title="Why this matters for snapshot tests"
The whole storybook snapshot harness depends on this being a synchronous, deterministic string render. `render_to_string` (older API) ran an executor; `to_html` walks the view eagerly. Don't switch to async helpers — the `tests/snapshots.rs` insta gate assumes sync.
```

---

## 7. Router

| Era | Crate |
| --- | --- |
| ≤ 0.6 | `leptos_router` with `<Routes><Route .. /></Routes>` |
| 0.7 – 0.8 ← strive | `leptos_router` — slightly different `Routes`/`Route` shape, supports nested + lazy + protected routes |
| 0.8 only ← strive | `islands-router` — client-side routing **inside** an islands app. See [examples/islands_router](https://github.com/leptos-rs/leptos/tree/main/examples/islands_router). |

This project doesn't currently use `leptos_router` — `app-ui` is a single screen. If/when it grows beyond one route, **start with the 0.8 islands-router** if the page is server-rendered, otherwise the standard router.

---

## 8. Server functions

The biggest 0.8 user-facing improvement.

### Custom error types — `FromServerFnError`

| Era | Custom error pattern |
| --- | --- |
| 0.7 | Stuff your custom enum into `ServerFnError::WrappedServerError` — clunky |
| 0.8 ← strive | Implement `FromServerFnError` directly on your error type — first-class |

```rust
#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub enum SaveError {
    #[error("invalid payload: {0}")]
    Invalid(String),
    #[error("storage failed: {0}")]
    Storage(String),
}

impl FromServerFnError for SaveError {
    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        Self::Storage(value.to_string())
    }
}

#[server]
async fn save(payload: MyData) -> Result<(), SaveError> { /* ... */ }
```

```admonish important
Match the URL-encoded error path: in 0.8, base64-encoded server-action error messages stored in the URL are correctly decoded (fix landed in v0.8.0). 0.7 had cases where they were double-encoded.
```

### Websocket server fns (0.8)

New in 0.8. Server fns can declare a `Websocket` protocol and accept/return a `BoxedStream`:

```rust
use server_fn::{codec::JsonEncoding, BoxedStream, ServerFnError, Websocket};

#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
async fn echo_ws(
    input: BoxedStream<String, ServerFnError>,
) -> Result<BoxedStream<String, ServerFnError>, ServerFnError> {
    // ...
}
```

If you reach for `tokio-tungstenite` directly, **stop and consider this primitive** — it removes nearly all manual websocket plumbing.

---

## 9. Stores (0.7+) — strive when state is a struct

Stores are the way to do nested reactive state with field-level subscribers.

```rust
use reactive_stores::Store;

#[derive(Store, Default)]
pub struct RecorderState {
    pub mode: CaptureMode,
    pub source: Option<DisplaySourceView>,
    pub mic: Option<DeviceFixture>,
}

let store = Store::new(RecorderState::default());
let mode_signal = store.mode();        // signal-like access to one field
let source_signal = store.source();
```

0.8 fixed a lot of store-field bugs that hit 0.7 (keyed-field patching, iteration over keyed collections, `track_caller` for store-field methods). **If you avoided stores on 0.7, give them another look in 0.8.**

---

## 10. Axum / SSR integration

Major 0.7 → 0.8 break: **axum bumped 0.7 → 0.8**.

| Era | Crate |
| --- | --- |
| 0.7 | `axum = "0.7"`, `leptos_axum = "0.7"` |
| 0.8 ← strive | `axum = "0.8"`, `leptos_axum = "0.8"` |

This is why 0.8 had to be a major release — `leptos_axum` re-exports axum types, so a major axum bump propagated. Axum 0.8 has its own breaking changes (notably path syntax: `/users/:id` → `/users/{id}`). When upgrading a Leptos server crate, **read axum 0.8's own changelog before the Leptos one.**

---

## 11. Build / development knobs — strive to enable

### `--cfg=erase_components` (0.8)

```sh
RUSTFLAGS="--cfg=erase_components" cargo build
```

Switches the view rendering machinery to use type-erased internals — **dramatically** faster compile times in dev mode. The latest `cargo-leptos` enables this by default. For Trunk-based projects (this is us), set it in `.cargo/config.toml`:

```toml
[build]
rustflags = ["--cfg=erase_components"]
```

```admonish tip
Pay the cost of erased components in **dev only**. For production / release builds, leave it off so you get the monomorphized fast-runtime version.
```

### Prelude

`use leptos::prelude::*;` brings everything modern into scope, including the `tachys::prelude::*` re-export that gives you `RenderHtml::to_html()`.

```admonish bug title="The old `leptos::*` glob is NOT the prelude"
On 0.7+ the entry point is `leptos::prelude::*`. `use leptos::*;` will compile but miss key trait imports (notably `RenderHtml`). When SSR rendering looks broken, **check that you said `prelude::*`.**
```

---

## 12. Quick-reference: name changes table

| Old (≤ 0.6) | New (0.7 – 0.8) | Notes |
| --- | --- | --- |
| `create_signal(cx, init)` | `signal(init)` or `RwSignal::new(init)` | drop `cx` |
| `create_memo(cx, fn)` | `Memo::new(\|_prev\| ...)` | closure takes prev |
| `create_effect(cx, fn)` | `Effect::new(fn)` / `Effect::watch(...)` | new `watch` form |
| `create_resource(cx, source, fetcher)` | `Resource::new(source, fetcher)` |  |
| `create_local_resource(cx, source, fetcher)` | `LocalResource::new(async fn)` | no SendWrapper in 0.8 |
| `create_action(cx, fn)` | `Action::new(fn)` |  |
| `fn Foo(cx: Scope, ...) -> impl IntoView` | `#[component] fn Foo(...) -> impl IntoView` | macro |
| `cx.children()` | `children: Children` slot prop |  |
| `ssr::render_to_string(...)` | `view.into_view().to_html()` | sync, walks eagerly |
| `ServerFnError::WrappedServerError` | `impl FromServerFnError for MyError` | first-class custom errors |
| `expect_context::<Scope>()` | — | scopes don't exist anymore |
| `provide_context(cx, value)` | `provide_context(value)` |  |
| `use_context::<T>(cx)` | `use_context::<T>()` |  |
| (no equivalent) | `Either!` macro (0.8) | replaces nested `<Show>` |
| (no equivalent) | `<ShowLet>` (0.8) | binds truthy value |
| (no equivalent) | `Websocket` server fn protocol (0.8) |  |
| (no equivalent) | `islands-router` (0.8) | client-side routing inside islands |

---

## 13. Project-specific landmines

These hit us on 0.7 and would hit again on 0.8 if not respected.

```admonish bug title="The `+y` flip — glyphon writes textures down, sprite samples up"
Not a Leptos issue, but every text-related Leptos component eventually ends up touching wisp via `RenderTexture::as_texture()`. The sprite needs `scale.y = -1`. This is documented in `_docs/book/src/wisp/text/textures.md`.
```

```admonish bug title="Leptos `#[component]` macro fires clippy on generated code"
Lints like `must_use_candidate` and `needless_pass_by_value` fire on the **builder struct + wrapper fn** the macro generates, regardless of where you place `#[allow]`. **Use module-level `#![allow(...)]` in `components/mod.rs`** rather than per-fn pragmas. This is documented in CLAUDE.md.
```

```admonish bug title="`Show`'s `when` closure must be `'static`"
If the closure reads a captured `String`, **clone a `bool` instead** and reconstruct the string inside the body. Captured borrows can't escape into the `view!` macro's closure.
```

```admonish bug title="Avoid `Some(ToChildren::to_children(...))`"
The `#[prop(optional)]` macro wraps the value in `Option`. Pass the bare `ToChildren::to_children(...)` for the slot value or omit the prop entirely. Wrapping in `Some` produces `Option<Option<Children>>` and you get a "expected `Box<dyn FnOnce()…>`, found `Option<_>`" error.
```

---

## 14. When updating an old example

The migration order that minimizes breakage:

1. **Bump the crate version**: `leptos = "0.X" → leptos = "0.8"`.
2. **Drop every `cx`** in fn signatures, calls, and provided context.
3. **Replace `create_*` constructors** per Section 12's table.
4. **Update `view.into_view().to_html()`** for any `render_to_string` SSR call.
5. **Remove `.as_deref()`** from `LocalResource` consumers.
6. **Re-check optional `Children`** slots — `Some(...)` wraps disappear.
7. `cargo check` until the type errors converge.
8. `cargo nextest run` — `insta` snapshots will likely need regeneration (Leptos hydration markers shift between versions; our snapshot test in `ui-storybook/tests/snapshots.rs` already strips `<!--hk=...-->` + `data-hk` attributes via `normalize`, so 0.7 → 0.8 should be near-identity, but accept the `*.snap.new` once if it diffs).

---

## 15. Reference: the actual change in this project's tree

This skill was authored alongside the `leptos-upgrade` branch that performed the 0.7 → 0.8 bump in `crates/{app-ui,ui-storybook}/Cargo.toml`. The diff was effectively a version bump + zero source-code edits because the project's component contract (stateless, controlled, props-only) sidesteps every breaking change in 0.8.

If a future upgrade does require source edits, the rule of thumb is:

```admonish important title="Tighten the contract, not the migration shim"
When a Leptos breaking change hits this codebase, the fix is almost always to lean **harder** into the presentational contract — push the state away from where the breaking change matters. Don't add `#[allow]` for the new lint; restructure so the lint can't fire.
```

---

*Authored 2026-05-11 alongside the `leptos-upgrade` branch (0.7 → 0.8). Update this file when a new major Leptos release lands; the next big one is likely 0.9 with reactive_stores improvements.*
