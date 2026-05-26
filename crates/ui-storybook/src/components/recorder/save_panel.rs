//! `SavePanel` (M-SAVE.GATE) — the post-record Save panel that
//! replaces the record/stop footer once a recording is parked
//! awaiting export.
//!
//! Two visual states, both rendered inside the same
//! `recorder-page-action-bar recorder-save-panel` footer so the panel
//! drops into the recorder column with no layout shift:
//!
//! - **Choosing** — a folder row (configured output dir + a Change…
//!   button), a format dropdown (`MP4` / `WebM`), and Discard / Export
//!   actions. The `busy` flag dims the controls and flips the Export
//!   label to "Exporting…" during the (software-`VP9`) `WebM` transcode.
//! - **Saved** — a "Saved to `<path>`" confirmation with Done /
//!   Reveal-in-Finder actions.
//!
//! Stateless: the parent (`app-ui`'s `RecorderPage`) owns the pending
//! export, the chosen format, the export-in-flight signal, and the
//! post-export saved path. It maps that state into a [`SavePanelView`]
//! and wires the optional `Callback<()>` props to the Tauri IPC
//! commands (`export_recording`, `discard_recording`,
//! `reveal_in_file_manager`, the output-dir picker). Stories leave the
//! callbacks unset.

use leptos::prelude::*;

/// Export container format offered by the Save panel.
///
/// The slugs match the recorder's IPC contract: `MP4`/H.264 is the
/// scratch's native format (export = a move), `WebM`/`VP9` transcodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SaveFormat {
    /// H.264 `MP4` — the scratch already is this, so export is a move.
    #[default]
    Mp4H264,
    /// `VP9`/`Opus` `WebM` — export transcodes (software `VP9`).
    WebmVp9,
}

impl SaveFormat {
    /// Stable slug used as the `<option value>` + the IPC format
    /// argument (`export_recording(format, …)`).
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            SaveFormat::Mp4H264 => "mp4-h264",
            SaveFormat::WebmVp9 => "webm-vp9",
        }
    }

    /// Short human label shown in the dropdown.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            SaveFormat::Mp4H264 => "MP4",
            SaveFormat::WebmVp9 => "WebM",
        }
    }

    /// Parse a slug back into a `SaveFormat`. Returns `None` for any
    /// unrecognised slug so the `on:change` handler can ignore it
    /// rather than guess.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "mp4-h264" => Some(SaveFormat::Mp4H264),
            "webm-vp9" => Some(SaveFormat::WebmVp9),
            _ => None,
        }
    }

    /// Every format the dropdown offers, in display order.
    #[must_use]
    pub fn all() -> [SaveFormat; 2] {
        [SaveFormat::Mp4H264, SaveFormat::WebmVp9]
    }
}

/// View-model for the Save panel — which of the two states to render
/// plus the data each needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SavePanelView {
    /// Pre-export: pick the folder + format, then Export or Discard.
    Choosing {
        /// Configured output directory, shown tail-visible.
        output_dir: String,
        /// Currently-selected export format.
        format: SaveFormat,
        /// `true` while an export is in flight — disables the controls
        /// and flips the Export label to "Exporting…".
        busy: bool,
    },
    /// Post-export: the file landed at `path`; offer Reveal / Done.
    Saved {
        /// Absolute path of the exported file.
        path: String,
    },
}

/// The post-record Save panel.
#[component]
pub fn SavePanel(
    /// View-model. The parent rebuilds this whenever the pending
    /// export, chosen format, busy flag, or saved path changes.
    view: SavePanelView,
    /// Change-folder click (Choosing state). Wired to the native
    /// folder picker.
    #[prop(optional, into)]
    on_change_folder: Option<Callback<()>>,
    /// Format-dropdown change (Choosing state). Receives the newly
    /// selected [`SaveFormat`].
    #[prop(optional, into)]
    on_format_change: Option<Callback<SaveFormat>>,
    /// Discard click (Choosing state) — deletes the scratch.
    #[prop(optional, into)]
    on_discard: Option<Callback<()>>,
    /// Export click (Choosing state) — runs the move / transcode.
    #[prop(optional, into)]
    on_export: Option<Callback<()>>,
    /// Reveal-in-Finder click (Saved state).
    #[prop(optional, into)]
    on_reveal: Option<Callback<()>>,
    /// Done click (Saved state) — dismisses the panel.
    #[prop(optional, into)]
    on_done: Option<Callback<()>>,
) -> impl IntoView {
    let body: AnyView = match view {
        SavePanelView::Choosing {
            output_dir,
            format,
            busy,
        } => choosing_body(
            output_dir,
            format,
            busy,
            on_change_folder,
            on_format_change,
            on_discard,
            on_export,
        ),
        SavePanelView::Saved { path } => saved_body(path, on_reveal, on_done),
    };
    view! {
        <footer class="recorder-page-action-bar recorder-save-panel" aria-label="Save recording">
            {body}
        </footer>
    }
}

/// The Choosing-state body: folder row + format dropdown + Discard /
/// Export actions. Split out of [`SavePanel`] so neither branch trips
/// the function-length lint; it carries no state (every prop is owned
/// or a `Copy` callback).
fn choosing_body(
    output_dir: String,
    format: SaveFormat,
    busy: bool,
    on_change_folder: Option<Callback<()>>,
    on_format_change: Option<Callback<SaveFormat>>,
    on_discard: Option<Callback<()>>,
    on_export: Option<Callback<()>>,
) -> AnyView {
    let dir_title = output_dir.clone();
    let change_click = move |_| {
        if let Some(cb) = on_change_folder {
            cb.run(());
        }
    };
    let discard_click = move |_| {
        if let Some(cb) = on_discard {
            cb.run(());
        }
    };
    let export_click = move |_| {
        if let Some(cb) = on_export {
            cb.run(());
        }
    };
    let options = SaveFormat::all()
        .into_iter()
        .map(|f| {
            view! {
                <option value=f.slug() selected=f == format>{f.label()}</option>
            }
        })
        .collect_view();
    let export_label = if busy { "Exporting…" } else { "Export" };
    view! {
        <div class="recorder-save-fields">
            <div class="recorder-save-row">
                <span class="recorder-save-key">"Folder"</span>
                <span class="recorder-save-folder" title=dir_title>{output_dir}</span>
                <button
                    type="button"
                    class="recorder-save-change"
                    on:click=change_click
                    disabled=busy
                >"Change…"</button>
            </div>
            <div class="recorder-save-row">
                <span class="recorder-save-key">"Format"</span>
                <select
                    class="recorder-save-format"
                    aria-label="Export format"
                    // `:target` (0.8) types `ev.target()` to the
                    // `<select>` so `.value()` needs no cast.
                    on:change:target=move |ev| {
                        if let Some(cb) = on_format_change
                            && let Some(fmt) = SaveFormat::from_slug(&ev.target().value())
                        {
                            cb.run(fmt);
                        }
                    }
                    disabled=busy
                >
                    {options}
                </select>
            </div>
        </div>
        <div class="recorder-save-actions">
            <button
                type="button"
                class="recorder-save-discard"
                on:click=discard_click
                disabled=busy
            >"Discard"</button>
            <button
                type="button"
                class="recorder-save-export"
                on:click=export_click
                disabled=busy
            >{export_label}</button>
        </div>
    }
    .into_any()
}

/// The Saved-state body: the "Saved to `<path>`" confirmation plus
/// Done / Reveal-in-Finder actions.
fn saved_body(
    path: String,
    on_reveal: Option<Callback<()>>,
    on_done: Option<Callback<()>>,
) -> AnyView {
    let path_title = path.clone();
    let done_click = move |_| {
        if let Some(cb) = on_done {
            cb.run(());
        }
    };
    let reveal_click = move |_| {
        if let Some(cb) = on_reveal {
            cb.run(());
        }
    };
    view! {
        <div class="recorder-save-saved" role="status" aria-live="polite">
            <span class="recorder-save-key">"Saved to"</span>
            <span class="recorder-save-folder" title=path_title>{path}</span>
        </div>
        <div class="recorder-save-actions">
            <button
                type="button"
                class="recorder-save-discard"
                on:click=done_click
            >"Done"</button>
            <button
                type="button"
                class="recorder-save-export"
                on:click=reveal_click
            >"Reveal in Finder"</button>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique_and_kebab() {
        let slugs: Vec<&str> = SaveFormat::all().iter().map(|f| f.slug()).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len(), "format slugs must be unique");
        for s in slugs {
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "slug `{s}` is not kebab-case",
            );
        }
    }

    #[test]
    fn from_slug_round_trips_every_variant() {
        for f in SaveFormat::all() {
            assert_eq!(SaveFormat::from_slug(f.slug()), Some(f));
        }
        assert_eq!(SaveFormat::from_slug("av1"), None);
        assert_eq!(SaveFormat::from_slug(""), None);
    }

    #[test]
    fn default_format_is_mp4() {
        // The panel defaults to MP4 (the scratch's native format —
        // export is a move, no transcode). Guards against a reorder of
        // the enum flipping the default.
        assert_eq!(SaveFormat::default(), SaveFormat::Mp4H264);
        assert_eq!(SaveFormat::default().slug(), "mp4-h264");
    }

    #[test]
    fn labels_are_non_empty_and_distinct() {
        let labels: Vec<&str> = SaveFormat::all().iter().map(|f| f.label()).collect();
        assert!(labels.iter().all(|l| !l.is_empty()));
        assert_ne!(labels[0], labels[1]);
    }
}
