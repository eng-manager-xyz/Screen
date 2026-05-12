//! Shell components — chrome that frames the application:
//! `AppShell` top-level layout, `NavigationRail` left-edge nav,
//! `WorkspaceBadge` + `UserAvatar` rail caps, `DropZone`, `StatusBar`.

pub mod app_shell;
pub mod drop_zone;
pub mod navigation_rail;
pub mod status_bar;
pub mod user_avatar;
pub mod workspace_badge;

pub use app_shell::AppShell;
pub use drop_zone::{DropZone, DropZoneState};
pub use navigation_rail::{AppSection, NavItemView, NavigationRail};
pub use status_bar::{StatusBar, StatusKind};
pub use user_avatar::{UserAvatar, UserAvatarView};
pub use workspace_badge::{WorkspaceBadge, WorkspaceBadgeView};
