//! Tauri-managed state for the system-audio capture session
//! (M-AUDIO-SYS.2 / AUT-282).
//!
//! Holds an `Option<SystemAudioStream>` behind a `Mutex` so the
//! four system-audio IPC commands share one source of truth: the
//! stream is `None` when idle, `Some` when capturing. The wrapper
//! mirrors [`crate::audio::MicCaptureHandle`]'s shape but the
//! ownership model is different — `SystemAudioStream` is itself
//! the active session (no separate worker thread), so the handle
//! is just the Option-wrapper.
//!
//! macOS-only — the underlying `media::sck_audio::SystemAudioStream`
//! is gated on `#[cfg(target_os = "macos")]`. Tauri commands that
//! consume this state are cfg-gated to match; non-macOS arms return
//! `SystemAudioError::NotMacOs`.

#![cfg(target_os = "macos")]

use std::sync::Mutex;

use media::sck_audio::{AudioAppFilter, SystemAudioConfig, SystemAudioError, SystemAudioStream};
use tauri::{AppHandle, Emitter};

/// Tauri-managed wrapper around the active system-audio capture
/// session. Held in `tauri::State`; the four `system_audio_*` IPC
/// commands all operate through it.
///
/// The wrapper also stores the last-requested [`AudioAppFilter`]
/// alongside the stream so it carries across the preview → recording
/// session swap — every `start_with_mixer` builds the new SCK
/// stream with the stored filter rather than the `AllAudio` default.
#[derive(Default)]
pub struct SystemAudioCaptureState {
    /// Active SCK session, `None` when idle.
    pub stream: Mutex<Option<SystemAudioStream>>,
    /// Last per-app filter requested via `set_filter`. Seeds every
    /// new session at construction time.
    pub filter: Mutex<AudioAppFilter>,
}

impl SystemAudioCaptureState {
    /// Start a fresh session. If one is already active it's dropped
    /// first (which calls Drop → stopCapture). Idempotent under
    /// concurrent calls — the mutex serialises.
    ///
    /// `app` is captured into the level-sink closure
    /// (M-AUDIO.METER / AUT-287) so the SCK delegate can emit
    /// `system-audio-level` events directly. `AppHandle` is
    /// `Send + Sync` and lightly cloneable; one capture per
    /// session.
    pub fn start(
        &self,
        app: &AppHandle,
        config: SystemAudioConfig,
    ) -> Result<(), SystemAudioError> {
        self.start_with_mixer(app, config, None)
    }

    /// Same as [`Self::start`] but accepts an optional
    /// `mixer_sink` (M-PIX.4) that pushes raw F32LE samples into
    /// the recorder's shared `AudioMixer` for the encoder feed
    /// thread to consume.
    pub fn start_with_mixer(
        &self,
        app: &AppHandle,
        config: SystemAudioConfig,
        mixer: Option<crate::recording::SharedAudioMixer>,
    ) -> Result<(), SystemAudioError> {
        let mut guard = self
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Drop the previous stream BEFORE creating the new one so
        // SCK isn't trying to run two sessions simultaneously.
        *guard = None;
        let app_for_sink = app.clone();
        let level_sink: media::sck_audio::LevelSink = Box::new(move |level: f32| {
            if let Err(err) = app_for_sink.emit("system-audio-level", level) {
                tracing::trace!(?err, "emit system-audio-level failed");
            }
        });
        let mixer_sink: Option<media::sck_audio::MixerSink> = mixer.map(|m| {
            let mixer_clone = m;
            let sink: media::sck_audio::MixerSink = Box::new(move |samples: &[f32]| {
                let mut guard = mixer_clone
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Err(err) = guard.push_sys_audio(samples) {
                    // Misalignment shouldn't happen — SCK emits
                    // aligned buffers — but warn instead of panic.
                    tracing::warn!(?err, "AudioMixer::push_sys_audio rejected SCK buffer");
                }
            });
            sink
        });
        // Clone + drop the filter lock before the (seconds-long) SCK
        // call so a concurrent `set_filter` isn't blocked on stream
        // construction.
        let stored_filter = self
            .filter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let stream = SystemAudioStream::new_with_sinks(
            config,
            Some(level_sink),
            mixer_sink,
            &stored_filter,
        )?;
        *guard = Some(stream);
        Ok(())
    }

    /// Stop the active session, if any. No-op when already stopped.
    /// The stored filter is preserved for the next session.
    pub fn stop(&self) {
        let mut guard = self
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
    }

    /// Store the per-app filter and push it to the active SCK stream
    /// if one is running. The stored value seeds the next session
    /// regardless of whether a stream is currently up.
    ///
    /// # Errors
    ///
    /// Returns the underlying SCK error if updating the active
    /// stream fails; the stored filter is still updated.
    pub fn set_filter(&self, filter: &AudioAppFilter) -> Result<(), SystemAudioError> {
        {
            let mut filter_guard = self
                .filter
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *filter_guard = filter.clone();
        }
        let guard = self
            .stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match guard.as_ref() {
            Some(stream) => stream.set_app_filter(filter),
            None => Ok(()),
        }
    }

    /// `true` if a session is currently held. Diagnostic / test
    /// helper; the picker UX uses a separate lifecycle signal.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.stream
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_inactive() {
        let state = SystemAudioCaptureState::default();
        assert!(!state.is_active());
    }

    #[test]
    fn stop_is_idempotent_when_inactive() {
        let state = SystemAudioCaptureState::default();
        state.stop();
        state.stop();
        assert!(!state.is_active());
    }

    fn stored_filter(state: &SystemAudioCaptureState) -> AudioAppFilter {
        state
            .filter
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    #[test]
    fn default_stored_filter_is_all_audio() {
        let state = SystemAudioCaptureState::default();
        assert_eq!(stored_filter(&state), AudioAppFilter::AllAudio);
    }

    #[test]
    fn set_filter_without_active_session_stores_for_next_start() {
        let state = SystemAudioCaptureState::default();
        let filter = AudioAppFilter::ExcludeApps(vec!["com.google.Chrome".into()]);
        state.set_filter(&filter).expect("no session → Ok");
        assert_eq!(stored_filter(&state), filter);
    }

    #[test]
    fn set_filter_overwrites_stored_value() {
        let state = SystemAudioCaptureState::default();
        let first = AudioAppFilter::OnlyApps(vec!["com.spotify.client".into()]);
        let second = AudioAppFilter::ExcludeApps(vec!["us.zoom.xos".into()]);
        state.set_filter(&first).unwrap();
        state.set_filter(&second).unwrap();
        assert_eq!(stored_filter(&state), second);
    }
}
