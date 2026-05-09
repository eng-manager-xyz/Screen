//! `playback` — player state machine that pumps decoded frames into wisp.
//!
//! Sits between [`decode::VideoStream`] (the upstream codec backend) and
//! `wisp::VideoTexture` (the GPU upload target). The shell (Tauri / native
//! winit) drives it via:
//!
//! - [`Player::play`] / [`Player::pause`] — transport control
//! - [`Player::tick(dt)`](Player::tick) — once per render frame; advances
//!   playback time and uploads any frames whose presentation timestamp has
//!   come due
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use decode::mock::MockVideoStream;
//! use playback::Player;
//! use pollster::block_on;
//! use wisp::application::{AppConfig, Application};
//!
//! let app = block_on(Application::new(AppConfig::default())).unwrap();
//! let stream = MockVideoStream::scrolling_gradient(640, 360, 90);
//! let mut player = Player::new(&app, Box::new(stream));
//! player.play();
//! player.tick(&app, Duration::from_millis(33));   // ~one source frame
//! ```

use std::time::Duration;

use decode::{VideoFrame, VideoStream};
use wisp::Texture;
use wisp::VideoTexture;
use wisp::application::Application;

/// Transport state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayState {
    /// No frames advance, but [`Player::texture`] still serves the last
    /// uploaded frame.
    #[default]
    Paused,
    /// `tick` advances `elapsed` and pulls frames as they come due.
    Playing,
    /// The stream has yielded `None`. Equivalent to `Paused` but distinct
    /// so the UI can swap "Pause" for "Replay" etc.
    Ended,
}

/// Player state machine. Owns the upstream stream and the GPU texture
/// holding the current frame.
pub struct Player {
    stream: Box<dyn VideoStream + Send>,
    texture: VideoTexture,
    state: PlayState,
    elapsed: Duration,
    /// Wallclock time at which the *next* frame in the stream becomes
    /// presentable. Initialised to zero so the first `tick` always pulls.
    next_due: Duration,
    last_uploaded_pts: Option<f64>,
    last_uploaded_index: Option<u64>,
}

impl Player {
    /// Build a player around an arbitrary [`VideoStream`].
    ///
    /// Allocates a `VideoTexture` matching the stream dimensions; that
    /// allocation is reused for every frame.
    #[must_use]
    pub fn new(app: &Application, stream: Box<dyn VideoStream + Send>) -> Self {
        let texture = VideoTexture::new(app, stream.width(), stream.height());
        Self {
            stream,
            texture,
            state: PlayState::Paused,
            elapsed: Duration::ZERO,
            next_due: Duration::ZERO,
            last_uploaded_pts: None,
            last_uploaded_index: None,
        }
    }

    /// Transition to [`PlayState::Playing`]. No-op if already playing or
    /// the stream has ended.
    pub fn play(&mut self) {
        if self.state == PlayState::Paused {
            self.state = PlayState::Playing;
        }
    }

    /// Transition to [`PlayState::Paused`]. No-op if not playing.
    pub fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
        }
    }

    /// Advance playback by `dt`. Pulls and uploads as many frames as have
    /// come due — typically zero or one per render tick, but this handles
    /// burst catch-up (multiple frames due) cleanly.
    ///
    /// No-op when paused / ended. Returns the number of frames uploaded
    /// so callers can drive a redraw signal off it.
    pub fn tick(&mut self, app: &Application, dt: Duration) -> u32 {
        if self.state != PlayState::Playing {
            return 0;
        }
        self.elapsed += dt;

        let mut uploaded = 0u32;
        while self.elapsed >= self.next_due {
            let Some(frame) = self.stream.next_frame() else {
                self.state = PlayState::Ended;
                break;
            };
            self.upload(app, &frame);
            let frame_dt = Duration::from_secs_f64(1.0 / f64::from(self.stream.frame_rate()));
            self.next_due += frame_dt;
            uploaded += 1;
        }
        uploaded
    }

    fn upload(&mut self, app: &Application, frame: &VideoFrame) {
        self.texture.upload_bgra(app, &frame.bgra);
        self.last_uploaded_pts = Some(frame.pts_seconds);
        self.last_uploaded_index = Some(frame.frame_index);
    }

    /// Borrow the current frame's GPU texture for use as a `Sprite` source.
    #[must_use]
    pub fn texture(&self) -> &Texture {
        self.texture.texture()
    }

    /// Current transport state.
    #[must_use]
    pub fn state(&self) -> PlayState {
        self.state
    }

    /// Total wallclock time advanced since `play()` first fired.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Presentation timestamp of the last uploaded frame, if any.
    #[must_use]
    pub fn last_uploaded_pts(&self) -> Option<f64> {
        self.last_uploaded_pts
    }

    /// Frame index of the last uploaded frame, if any.
    #[must_use]
    pub fn last_uploaded_index(&self) -> Option<u64> {
        self.last_uploaded_index
    }

    /// Duration hint, derived from the stream's frame count + frame rate.
    #[must_use]
    pub fn duration_hint(&self) -> Option<Duration> {
        let count = self.stream.frame_count_hint()?;
        #[allow(
            clippy::cast_precision_loss,
            reason = "frame counts in practice fit comfortably in f64's 52-bit mantissa (2^52 frames at 60 fps ≈ 2.4 million years)"
        )]
        let count_f = count as f64;
        Some(Duration::from_secs_f64(
            count_f / f64::from(self.stream.frame_rate()),
        ))
    }
}
