//! Control-primitive stories (M-UI.4 / AUT-124).

use leptos::prelude::*;

use crate::components::primitives::{
    ColorSwatch, IconButton, IconButtonVariant, Meter, Segment, SegmentedControl, SelectPill,
    Slider, ToggleSwitch,
};

use super::{Story, StoryViewport, render};

/// All control-primitive stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls-buttons-icon-buttons",
            category: "Primitives",
            title: "Controls — icon buttons",
            viewport: StoryViewport::Auto,
            render: render_icon_buttons,
        },
        Story {
            id: "controls-toggle-states",
            category: "Primitives",
            title: "Controls — toggle switch states",
            viewport: StoryViewport::Auto,
            render: render_toggle_states,
        },
        Story {
            id: "controls-segmented-record-mode",
            category: "Primitives",
            title: "Controls — segmented record mode",
            viewport: StoryViewport::Auto,
            render: render_segmented,
        },
        Story {
            id: "controls-slider-values",
            category: "Primitives",
            title: "Controls — slider matrix",
            viewport: StoryViewport::Auto,
            render: render_sliders,
        },
        Story {
            id: "controls-color-swatches",
            category: "Primitives",
            title: "Controls — color swatches",
            viewport: StoryViewport::Auto,
            render: render_color_swatches,
        },
        Story {
            id: "controls-meters",
            category: "Primitives",
            title: "Controls — audio level meters",
            viewport: StoryViewport::Auto,
            render: render_meters,
        },
    ]
}

fn render_icon_buttons() -> String {
    render(view! {
        <div class="story-row" style="padding:14px;gap:14px">
            <IconButton glyph="●" label="Record" variant=IconButtonVariant::Danger />
            <IconButton glyph="◼" label="Stop" variant=IconButtonVariant::Filled />
            <IconButton glyph="↻" label="Reset" variant=IconButtonVariant::Ghost />
            <IconButton glyph="✱" label="Pinned" variant=IconButtonVariant::Ghost pressed=true />
            <IconButton glyph="⌫" label="Delete" variant=IconButtonVariant::Ghost disabled=true />
        </div>
    })
}

fn render_toggle_states() -> String {
    render(view! {
        <div class="story-row" style="padding:14px;gap:20px">
            <div class="story-row" style="flex-direction:column;align-items:center;gap:6px">
                <ToggleSwitch checked=false label="off" />
                <span style="font-size:11px;color:var(--text-tertiary)">"Off"</span>
            </div>
            <div class="story-row" style="flex-direction:column;align-items:center;gap:6px">
                <ToggleSwitch checked=true label="on" />
                <span style="font-size:11px;color:var(--text-tertiary)">"On"</span>
            </div>
            <div class="story-row" style="flex-direction:column;align-items:center;gap:6px">
                <ToggleSwitch checked=false disabled=true label="disabled off" />
                <span style="font-size:11px;color:var(--text-tertiary)">"Disabled off"</span>
            </div>
            <div class="story-row" style="flex-direction:column;align-items:center;gap:6px">
                <ToggleSwitch checked=true disabled=true label="disabled on" />
                <span style="font-size:11px;color:var(--text-tertiary)">"Disabled on"</span>
            </div>
        </div>
    })
}

fn render_segmented() -> String {
    let segments = vec![
        Segment {
            id: "screen",
            label: "Screen",
            icon: Some("▢"),
            disabled: false,
        },
        Segment {
            id: "window",
            label: "Window",
            icon: Some("◫"),
            disabled: false,
        },
        Segment {
            id: "area",
            label: "Area",
            icon: Some("⊞"),
            disabled: false,
        },
    ];
    render(view! {
        <div style="padding:14px;display:flex;flex-direction:column;gap:14px;align-items:flex-start">
            <SegmentedControl segments=segments.clone() active="screen" label="Capture mode" />
            <SegmentedControl segments=segments.clone() active="window" label="Capture mode (window)" />
            <SegmentedControl segments=segments active="area" label="Capture mode (area)" />
        </div>
    })
}

fn render_sliders() -> String {
    render(view! {
        <div style="padding:14px;display:flex;flex-direction:column;gap:14px;width:360px">
            <Slider value=0.0 label="Min".to_string() readout="0%".to_string() />
            <Slider value=0.42 label="Volume".to_string() readout="42%".to_string() />
            <Slider value=1.0 label="Max".to_string() readout="100%".to_string() />
            <Slider value=0.5 min=0.0 max=10.0 label="Disabled".to_string() readout="5.0".to_string() disabled=true />
        </div>
    })
}

fn render_color_swatches() -> String {
    render(view! {
        <div class="story-row" style="padding:14px;gap:10px">
            <ColorSwatch color="#fafafa".to_string() label="White".to_string() selected=true />
            <ColorSwatch color="#ef4444".to_string() label="Red".to_string() />
            <ColorSwatch color="#f59e0b".to_string() label="Amber".to_string() />
            <ColorSwatch color="#10b981".to_string() label="Emerald".to_string() />
            <ColorSwatch color="#38bdf8".to_string() label="Sky".to_string() />
            <ColorSwatch color="#a78bfa".to_string() label="Violet".to_string() />
            <ColorSwatch color="#18181b".to_string() label="Zinc".to_string() />
        </div>
    })
}

fn render_meters() -> String {
    render(view! {
        <div style="padding:14px;display:flex;flex-direction:column;gap:10px;width:280px">
            <div class="story-row" style="gap:10px">
                <span style="width:60px;font-size:11px;color:var(--text-secondary)">"Silent"</span>
                <Meter level=0.0 />
            </div>
            <div class="story-row" style="gap:10px">
                <span style="width:60px;font-size:11px;color:var(--text-secondary)">"Low"</span>
                <Meter level=0.25 />
            </div>
            <div class="story-row" style="gap:10px">
                <span style="width:60px;font-size:11px;color:var(--text-secondary)">"Mid"</span>
                <Meter level=0.6 />
            </div>
            <div class="story-row" style="gap:10px">
                <span style="width:60px;font-size:11px;color:var(--text-secondary)">"Loud"</span>
                <Meter level=0.85 />
            </div>
            <div class="story-row" style="gap:10px">
                <span style="width:60px;font-size:11px;color:var(--text-secondary)">"Clipping"</span>
                <Meter level=1.0 danger=true />
            </div>
            <SelectPill icon="🎙".to_string() label="Built-in microphone".to_string() />
        </div>
    })
}
