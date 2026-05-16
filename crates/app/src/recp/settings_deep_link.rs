//! M-RECP.0 / AUT-261 — System Settings deep-link helpers.
//!
//! Maps a [`SettingsPane`] enum to the OS-specific URL / shell-command
//! the user's system honours. Today: macOS + Windows return real URLs;
//! Linux returns `None` (no universal deep-link).

/// Which System Settings / Control Panel pane to open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPane {
    /// Camera privacy pane (Privacy & Security → Camera on macOS,
    /// Privacy → Camera on Windows 11).
    Camera,
    /// Microphone privacy pane.
    Microphone,
    /// Screen recording privacy pane (macOS only — Windows doesn't
    /// have this as a system-level Settings pane).
    ScreenRecording,
}

/// The shell argument list that opens the requested pane on the
/// current OS, OR `None` if no deep-link is known. Linux returns
/// `None` because the desktop environment determines the right
/// command (GNOME Control Center vs KDE System Settings vs …).
#[must_use]
pub fn open_command(pane: SettingsPane) -> Option<Vec<String>> {
    let url = url_for_current_os(pane)?;
    Some(open_args_for_current_os(&url))
}

#[cfg(target_os = "macos")]
#[must_use]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the macOS / Windows / other-OS variants share one Option<String> signature; macos always returns Some but the cross-OS callers branch on None"
)]
fn url_for_current_os(pane: SettingsPane) -> Option<String> {
    Some(
        match pane {
            SettingsPane::Camera => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera"
            }
            SettingsPane::Microphone => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            SettingsPane::ScreenRecording => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
            }
        }
        .to_string(),
    )
}

#[cfg(target_os = "windows")]
fn url_for_current_os(pane: SettingsPane) -> Option<String> {
    Some(
        match pane {
            SettingsPane::Camera => "ms-settings:privacy-webcam",
            SettingsPane::Microphone => "ms-settings:privacy-microphone",
            SettingsPane::ScreenRecording => return None,
        }
        .to_string(),
    )
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn url_for_current_os(_pane: SettingsPane) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn open_args_for_current_os(url: &str) -> Vec<String> {
    vec!["open".into(), url.into()]
}

#[cfg(target_os = "windows")]
fn open_args_for_current_os(url: &str) -> Vec<String> {
    vec!["cmd".into(), "/c".into(), "start".into(), url.into()]
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_args_for_current_os(_url: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_returns_real_camera_url() {
        let cmd = open_command(SettingsPane::Camera).unwrap();
        assert_eq!(cmd[0], "open");
        assert!(cmd[1].contains("Privacy_Camera"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_supports_all_three_panes() {
        for pane in [
            SettingsPane::Camera,
            SettingsPane::Microphone,
            SettingsPane::ScreenRecording,
        ] {
            assert!(open_command(pane).is_some(), "missing url for {pane:?}");
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_supports_camera_and_microphone() {
        assert!(open_command(SettingsPane::Camera).is_some());
        assert!(open_command(SettingsPane::Microphone).is_some());
        // Windows has no system-level screen-recording pane.
        assert!(open_command(SettingsPane::ScreenRecording).is_none());
    }
}
