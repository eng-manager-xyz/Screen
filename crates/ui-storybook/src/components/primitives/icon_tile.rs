//! `IconTile` — small square tile holding an icon or monogram
//! (M-UI.1 / AUT-121).
//!
//! Five kinds for the recurring tile usages: workspace monogram, device
//! avatar (mic / camera), app icon, action chip, user avatar.

use leptos::prelude::*;

/// Semantic icon-tile kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconTileKind {
    /// Workspace monogram — 2-letter initials over a neutral background.
    #[default]
    Workspace,
    /// Device avatar — microphone / camera / display row entries.
    Device,
    /// App icon — a system app emitting audio or window-capture target.
    App,
    /// Action tile — neutral chrome that hosts a leading glyph in a
    /// menu row.
    Action,
    /// User avatar — square with rounded corners.
    User,
}

impl IconTileKind {
    /// CSS class for the kind.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            IconTileKind::Workspace => "icon-tile-workspace",
            IconTileKind::Device => "icon-tile-device",
            IconTileKind::App => "icon-tile-app",
            IconTileKind::Action => "icon-tile-action",
            IconTileKind::User => "icon-tile-user",
        }
    }
}

#[component]
pub fn IconTile(
    #[prop(optional)] kind: IconTileKind,
    #[prop(optional, into)] extra_class: String,
    children: Children,
) -> impl IntoView {
    let class = format!(
        "icon-tile {}{}{}",
        kind.css(),
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! { <span class=class aria-hidden="true">{children()}</span> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_unique_class() {
        let classes = [
            IconTileKind::Workspace.css(),
            IconTileKind::Device.css(),
            IconTileKind::App.css(),
            IconTileKind::Action.css(),
            IconTileKind::User.css(),
        ];
        let mut sorted = classes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), classes.len());
    }

    #[test]
    fn each_class_starts_with_icon_tile_prefix() {
        for k in [
            IconTileKind::Workspace,
            IconTileKind::Device,
            IconTileKind::App,
            IconTileKind::Action,
            IconTileKind::User,
        ] {
            assert!(k.css().starts_with("icon-tile-"));
        }
    }
}
