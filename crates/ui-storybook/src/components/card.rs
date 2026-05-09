//! Card — surface container with optional header + body slots.
//!
//! Mirrors rust-ui's `<Card>` / `<CardHeader>` / `<CardContent>` composition
//! pattern (we use `CardBody` for the content slot to keep the prop name
//! short).

use leptos::prelude::*;

#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! {
        <div class="card">
            {children()}
        </div>
    }
}

#[component]
pub fn CardHeader(
    #[prop(into)] title: String,
    #[prop(optional, into)] subtitle: String,
) -> impl IntoView {
    let has_subtitle = !subtitle.is_empty();
    view! {
        <div class="card-header">
            <div class="card-title">{title}</div>
            <Show when=move || has_subtitle>
                <div class="card-subtitle">{subtitle.clone()}</div>
            </Show>
        </div>
    }
}

#[component]
pub fn CardBody(children: Children) -> impl IntoView {
    view! {
        <div class="card-body">
            {children()}
        </div>
    }
}
