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
#[derive(Default)]
pub struct SystemAudioCaptureState(pub Mutex<Option<SystemAudioStream>>);

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
            .0
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
        let stream = SystemAudioStream::new_with_sinks(config, Some(level_sink), mixer_sink)?;
        *guard = Some(stream);
        Ok(())
    }

    /// Stop the active session, if any. No-op when already stopped.
    pub fn stop(&self) {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = None;
    }

    /// Apply a new per-app filter to the active session. Returns
    /// `Err(NoActiveSession)` if no session is up — callers should
    /// `start` first then apply the filter.
    pub fn set_filter(&self, filter: &AudioAppFilter) -> Result<(), SystemAudioError> {
        let guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match guard.as_ref() {
            Some(stream) => stream.set_app_filter(filter),
            None => Err(SystemAudioError::StartFailed(
                "no active system-audio session to apply filter to".into(),
            )),
        }
    }

    /// `true` if a session is currently held. Diagnostic / test
    /// helper; the picker UX uses a separate lifecycle signal.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.0
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

    #[test]
    fn set_filter_without_active_session_errors() {
        let state = SystemAudioCaptureState::default();
        let err = state.set_filter(&AudioAppFilter::AllAudio).unwrap_err();
        assert!(matches!(err, SystemAudioError::StartFailed(_)));
    }
}
