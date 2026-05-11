//! `StatusBar` — slim bottom-of-app strip with engine-level health.
//!
//! Three pieces of information across, all driven by props the parent ticks:
//! FPS (renderer), encoder state, file size on disk. A single `kind: StatusKind`
//! drives the right-hand pill ("Ready", "Encoding", "Error") and its color.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusKind {
    #[default]
    Ready,
    Busy,
    Error,
}

impl StatusKind {
    fn css(self) -> &'static str {
        match self {
            StatusKind::Ready => "status-pill-ready",
            StatusKind::Busy => "status-pill-busy",
            StatusKind::Error => "status-pill-error",
        }
    }

    fn label(self) -> &'static str {
        match self {
            StatusKind::Ready => "Ready",
            StatusKind::Busy => "Working",
            StatusKind::Error => "Error",
        }
    }
}

#[component]
pub fn StatusBar(
    /// Renderer FPS, formatted as a whole number.
    #[prop(optional)]
    fps: f32,
    /// Encoder one-line status (e.g. `"H.264 · 9.4 Mbps"`).
    #[prop(optional, into)]
    encoder: String,
    /// File size on disk for the active recording, in bytes.
    #[prop(optional)]
    file_bytes: u64,
    /// Right-hand health pill kind.
    #[prop(optional)]
    kind: StatusKind,
    /// Free-text detail next to the pill (e.g. error message).
    #[prop(optional, into)]
    detail: String,
) -> impl IntoView {
    let fps_value = fps.max(0.0).round();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "fps is positive and bounded; rendered FPS comfortably fits in u32"
    )]
    let fps_int = fps_value as u32;
    let encoder = if encoder.is_empty() {
        "—".to_string()
    } else {
        encoder
    };
    let size = format_bytes(file_bytes);
    let pill_class = format!("status-pill {}", kind.css());
    let pill_label = kind.label();
    let has_detail = !detail.is_empty();

    view! {
        <div class="status-bar" role="contentinfo" aria-label="Engine status">
            <div class="status-cell">
                <span class="status-key">"FPS"</span>
                <span class="status-value">{fps_int}</span>
            </div>
            <div class="status-cell">
                <span class="status-key">"Encoder"</span>
                <span class="status-value">{encoder}</span>
            </div>
            <div class="status-cell">
                <span class="status-key">"Size"</span>
                <span class="status-value">{size}</span>
            </div>
            <div class="status-spacer"></div>
            <Show when=move || has_detail>
                <span class="status-detail">{detail.clone()}</span>
            </Show>
            <span class=pill_class>
                <span class="status-pill-dot"></span>
                {pill_label}
            </span>
        </div>
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes == 0 {
        return "0 B".into();
    }
    if bytes >= GB {
        #[allow(
            clippy::cast_precision_loss,
            reason = "display value, fractional precision sufficient"
        )]
        let v = bytes as f64 / GB as f64;
        format!("{v:.2} GB")
    } else if bytes >= MB {
        #[allow(
            clippy::cast_precision_loss,
            reason = "display value, fractional precision sufficient"
        )]
        let v = bytes as f64 / MB as f64;
        format!("{v:.1} MB")
    } else if bytes >= KB {
        #[allow(
            clippy::cast_precision_loss,
            reason = "display value, fractional precision sufficient"
        )]
        let v = bytes as f64 / KB as f64;
        format!("{v:.0} KB")
    } else {
        format!("{bytes} B")
    }
}
