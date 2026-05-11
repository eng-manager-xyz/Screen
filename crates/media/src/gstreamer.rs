//! Shared GStreamer probe — structured diagnostic for capture/playback
//! preconditions (M-MEDIA.1 / AUT-97).
//!
//! [`probe`] runs the actual checks; the returned [`GStreamerProbe`] is
//! the structured form (`Option<version>` per binary, requested-plugin
//! map, `PATH` snapshot for CI diagnosis). [`is_available`] is the
//! `bool` shortcut callers use as a skip-guard for integration tests.
//!
//! # Why this exists
//!
//! Capture / playback paths all assume `gst-launch-1.0` and
//! `gst-discoverer-1.0` are on `PATH`. CI on Linux apt-installs
//! `gstreamer1.0-tools`, but per CLAUDE.md's "GStreamer / CI" lessons
//! some nextest processes still get `ENOENT` on spawn — the cause is
//! unclear, but the skip guard makes it a non-issue.
//!
//! Structured diagnostic (vs the old `bool` helper) means the failure
//! message can say *why* — `gst-launch-1.0 missing, gst-discoverer-1.0
//! present, PATH=/usr/bin:/usr/local/bin:…` — instead of just "false."
//!
//! # Quick start
//!
//! ```no_run
//! use media::gstreamer::{is_available, probe};
//!
//! if !is_available() {
//!     eprintln!("skipping GStreamer test:\n{}", probe());
//!     return;
//! }
//! // …run the integration test…
//! ```

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

/// Structured GStreamer probe result.
///
/// `gst_launch` / `gst_discoverer` contain the trimmed `--version`
/// output when the binary is callable, or `None` when the spawn fails.
/// `plugins` is the requested-plugin map (from
/// [`probe_with_plugins`]); empty when [`probe`] is used.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GStreamerProbe {
    /// `gst-launch-1.0 --version` first line, or `None` if the spawn
    /// failed (binary missing, permission denied, etc.).
    pub gst_launch: Option<String>,
    /// `gst-discoverer-1.0 --version` first line, or `None`.
    pub gst_discoverer: Option<String>,
    /// Requested-plugin presence map (plugin-name → present?). Filled
    /// only by [`probe_with_plugins`]. Sorted by name for deterministic
    /// output.
    pub plugins: BTreeMap<String, bool>,
    /// Snapshot of `PATH` at probe time. Useful in CI logs when a
    /// previously-working install starts spawning `ENOENT`.
    pub path: String,
}

impl GStreamerProbe {
    /// True when both `gst-launch-1.0` and `gst-discoverer-1.0` are
    /// callable. Plugin-presence is not considered — explicit
    /// plugin requirements should be checked via [`Self::has_plugin`].
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.gst_launch.is_some() && self.gst_discoverer.is_some()
    }

    /// True when the named plugin was checked AND present. Returns
    /// `false` for both "checked-but-missing" and "not-checked."
    /// Callers wanting to distinguish should inspect [`Self::plugins`]
    /// directly.
    #[must_use]
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.get(name).copied().unwrap_or(false)
    }
}

impl std::fmt::Display for GStreamerProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "GStreamer probe:\n  gst-launch-1.0      = {}\n  gst-discoverer-1.0  = {}",
            self.gst_launch
                .as_deref()
                .map_or_else(|| "<missing>".to_owned(), str::to_owned),
            self.gst_discoverer
                .as_deref()
                .map_or_else(|| "<missing>".to_owned(), str::to_owned),
        )?;
        if !self.plugins.is_empty() {
            writeln!(f, "  plugins:")?;
            for (name, present) in &self.plugins {
                writeln!(
                    f,
                    "    {name:24} = {}",
                    if *present { "present" } else { "<missing>" }
                )?;
            }
        }
        write!(f, "  PATH                = {}", self.path)
    }
}

/// Probe both CLIs without checking any plugins.
#[must_use]
pub fn probe() -> GStreamerProbe {
    probe_with_plugins(&[])
}

/// Probe both CLIs and check the named plugins via `gst-inspect-1.0`.
///
/// `gst-inspect-1.0 <plugin>` exits 0 when the plugin is registered.
/// If `gst-inspect-1.0` itself isn't on `PATH`, every requested
/// plugin entry comes back as `false`.
#[must_use]
pub fn probe_with_plugins(plugins: &[&str]) -> GStreamerProbe {
    let gst_launch = version_of("gst-launch-1.0");
    let gst_discoverer = version_of("gst-discoverer-1.0");

    let mut plugin_map = BTreeMap::new();
    for &p in plugins {
        plugin_map.insert(p.to_owned(), check_plugin(p));
    }

    GStreamerProbe {
        gst_launch,
        gst_discoverer,
        plugins: plugin_map,
        path: std::env::var("PATH").unwrap_or_else(|_| "<unset>".to_owned()),
    }
}

/// Shortcut: `probe().is_available()`.
///
/// Use this in integration tests as a skip-guard so the test
/// gracefully no-ops when GStreamer isn't installed.
///
/// ```no_run
/// if !media::gstreamer::is_available() {
///     eprintln!("skipping: GStreamer not on PATH");
///     return;
/// }
/// ```
#[must_use]
pub fn is_available() -> bool {
    probe().is_available()
}

fn version_of(cmd: &str) -> Option<String> {
    let output = Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    Some(s.lines().next().unwrap_or("").trim().to_owned())
}

fn check_plugin(plugin: &str) -> bool {
    Command::new("gst-inspect-1.0")
        .arg(plugin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_path_snapshot() {
        let p = probe();
        // PATH may legitimately be empty, but the snapshot field is
        // always populated (with "<unset>" if env var is missing).
        assert!(!p.path.is_empty());
    }

    #[test]
    fn probe_plugins_is_empty_when_none_requested() {
        let p = probe();
        assert!(p.plugins.is_empty());
    }

    #[test]
    fn probe_with_plugins_records_requested_names() {
        // Use an obviously-unknown plugin so the result is deterministic
        // regardless of whether GStreamer is installed: it'll come back
        // as false either way.
        let p = probe_with_plugins(&["__definitely_not_a_real_plugin__"]);
        assert_eq!(p.plugins.len(), 1);
        assert!(!p.has_plugin("__definitely_not_a_real_plugin__"));
        // Not requested → not present.
        assert!(!p.has_plugin("videoconvert"));
    }

    #[test]
    fn is_available_matches_probe_is_available() {
        let direct = is_available();
        let via_probe = probe().is_available();
        assert_eq!(direct, via_probe);
    }

    #[test]
    fn display_reports_missing_when_binary_absent() {
        // Build a synthetic probe by hand — independent of host state.
        let synthetic = GStreamerProbe {
            gst_launch: None,
            gst_discoverer: Some("gst-discoverer-1.0 version 1.26.8".to_owned()),
            plugins: BTreeMap::new(),
            path: "/usr/bin".to_owned(),
        };
        let rendered = format!("{synthetic}");
        assert!(rendered.contains("<missing>"));
        assert!(rendered.contains("gst-discoverer-1.0 version 1.26.8"));
        assert!(rendered.contains("/usr/bin"));
        assert!(!synthetic.is_available());
    }

    #[test]
    fn probe_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GStreamerProbe>();
    }
}
