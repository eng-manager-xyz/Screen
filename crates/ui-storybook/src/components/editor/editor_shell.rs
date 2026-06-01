//! `EditorShell` + top toolbar (M-UI.16 / AUT-136). Structural shell
//! for the editor screen — title bar, top toolbar, and slot regions
//! for the canvas / inspector / timeline.
//!
//! All slots are optional `Children` so stories can demonstrate the
//! empty shell, the no-clip state, and the full composition.

use leptos::prelude::*;

/// View-model for one toolbar action chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarActionView {
    /// Stable id ("16:9", "crop", "annotate").
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// Leading glyph.
    pub icon: &'static str,
    /// `true` when active.
    pub selected: bool,
    /// `true` for disabled actions.
    pub disabled: bool,
}

/// Shell view-model.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorShellView {
    /// Document title shown center-top.
    pub document_title: String,
    /// Optional subtitle ("Captured 2026-05-09 · 1m 24s").
    pub document_subtitle: Option<String>,
    /// `false` to render the "no clip" placeholder hint in the title.
    pub has_clip_loaded: bool,
    /// Toolbar actions in display order.
    pub toolbar_actions: Vec<ToolbarActionView>,
    /// `true` to enable the Export button.
    pub export_enabled: bool,
    /// `true` to enable the Share button.
    pub share_enabled: bool,
}

#[component]
pub fn EditorTitleBar(
    /// Document title.
    #[prop(into)]
    title: String,
    /// Optional subtitle.
    #[prop(optional, into)]
    subtitle: String,
    /// `false` renders a "No clip loaded" hint in place of the title.
    has_clip_loaded: bool,
) -> impl IntoView {
    let title_text = if has_clip_loaded {
        title
    } else {
        "No clip loaded".to_owned()
    };
    let class = if has_clip_loaded {
        "editor-titlebar"
    } else {
        "editor-titlebar editor-titlebar-empty"
    };
    let subtitle_view = (!subtitle.is_empty() && has_clip_loaded)
        .then(|| view! { <span class="editor-titlebar-subtitle">{subtitle}</span> });
    view! {
        <header class=class>
            <span class="editor-titlebar-traffic" aria-hidden="true">
                <span class="traffic-dot traffic-close"></span>
                <span class="traffic-dot traffic-min"></span>
                <span class="traffic-dot traffic-max"></span>
            </span>
            <span class="editor-titlebar-text">
                <span class="editor-titlebar-title">{title_text}</span>
                {subtitle_view}
            </span>
        </header>
    }
}

#[component]
pub fn EditorToolbar(
    /// Toolbar actions.
    actions: Vec<ToolbarActionView>,
    /// `true` to enable the Export button.
    export_enabled: bool,
    /// `true` to enable the Share button.
    share_enabled: bool,
    /// Fired with the chip's `id` when a (non-disabled) toolbar action is
    /// clicked. The app maps the id to an edit op; the component stays
    /// presentational (it only emits the id). Omit to leave chips inert.
    /// Plain `optional` (not `into`) so [`EditorShell`] can forward its own
    /// already-`Option` value straight through.
    #[prop(optional)]
    on_action: Option<Callback<String>>,
) -> impl IntoView {
    let action_chips: Vec<_> = actions
        .into_iter()
        .map(|a| {
            let mut class = String::from("editor-action");
            if a.selected {
                class.push_str(" editor-action-selected");
            }
            if a.disabled {
                class.push_str(" editor-action-disabled");
            }
            let id = a.id;
            let label = a.label;
            let icon = a.icon;
            let disabled = a.disabled;
            let pressed = a.selected;
            view! {
                <button
                    class=class
                    data-id=id
                    disabled=disabled
                    aria-pressed=pressed
                    on:click=move |_| {
                        if let Some(cb) = on_action {
                            cb.run(id.to_owned());
                        }
                    }
                >
                    <span class="editor-action-icon" aria-hidden="true">{icon}</span>
                    <span class="editor-action-label">{label}</span>
                </button>
            }
        })
        .collect();
    view! {
        <div class="editor-toolbar" role="toolbar" aria-label="Editor tools">
            <div class="editor-toolbar-actions">{action_chips}</div>
            <div class="editor-toolbar-end">
                <button class="btn btn-outline btn-sm" disabled=!share_enabled>"Share"</button>
                <button class="btn btn-default btn-sm" disabled=!export_enabled>"Export"</button>
            </div>
        </div>
    }
}

#[component]
pub fn EditorShell(
    /// View-model.
    view: EditorShellView,
    /// Center canvas slot (drop zone, video preview, etc.).
    #[prop(optional)]
    canvas: Option<Children>,
    /// Right inspector slot.
    #[prop(optional)]
    inspector: Option<Children>,
    /// Bottom timeline slot.
    #[prop(optional)]
    timeline: Option<Children>,
    /// Forwarded to [`EditorToolbar`]: fired with a toolbar chip's `id` on
    /// click. Omit to leave the toolbar inert.
    #[prop(optional, into)]
    on_action: Option<Callback<String>>,
) -> impl IntoView {
    let EditorShellView {
        document_title,
        document_subtitle,
        has_clip_loaded,
        toolbar_actions,
        export_enabled,
        share_enabled,
    } = view;
    view! {
        <section class="editor-shell" role="main" aria-label="Editor">
            <EditorTitleBar
                title=document_title
                subtitle=document_subtitle.unwrap_or_default()
                has_clip_loaded=has_clip_loaded
            />
            <EditorToolbar
                actions=toolbar_actions
                export_enabled=export_enabled
                share_enabled=share_enabled
                on_action=on_action.unwrap_or_else(|| Callback::new(|_: String| {}))
            />
            <div class="editor-body">
                <div class="editor-canvas">
                    {canvas.map(|c| c())}
                </div>
                <aside class="editor-inspector" aria-label="Inspector">
                    {inspector.map(|i| i())}
                </aside>
            </div>
            <footer class="editor-timeline">
                {timeline.map(|t| t())}
            </footer>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_titlebar_is_marked() {
        let v = EditorShellView {
            document_title: "Demo".into(),
            document_subtitle: None,
            has_clip_loaded: false,
            toolbar_actions: vec![],
            export_enabled: false,
            share_enabled: false,
        };
        assert!(!v.has_clip_loaded);
    }
}
