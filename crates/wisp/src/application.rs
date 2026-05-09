//! `Application` — root context for wisp, owns the wgpu device and queue.
//!
//! Headless only in M0.4; surface support lands in M0.5.

use wgpu::{Adapter, Device, Instance, Queue};

use crate::Error;
use crate::scene::Stage;

/// Runtime configuration passed to [`Application::new`].
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Logical width of the stage in pixels at DPR 1×.
    pub width: u32,
    /// Logical height of the stage in pixels at DPR 1×.
    pub height: u32,
    /// GPU power preference. Defaults to `HighPerformance`.
    pub power_preference: wgpu::PowerPreference,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            power_preference: wgpu::PowerPreference::HighPerformance,
        }
    }
}

/// Root wgpu context.
///
/// Owns the `Instance`, `Adapter`, `Device`, and `Queue`. Construct with
/// [`Application::new`] (async, await with a runtime such as `pollster`).
///
/// Headless only in M0.4 — surface attachment lands in M0.5.
pub struct Application {
    config: AppConfig,
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    stage: Stage,
}

impl Application {
    /// Adopt a pre-existing wgpu context (e.g. from `eframe`).
    ///
    /// All four wgpu resources are cloned into the `Application`; `wgpu` types
    /// are `Arc`-backed so this is cheap. Use this instead of [`Application::new`]
    /// when integrating with an embedding host that owns the device.
    #[must_use]
    pub fn from_wgpu(
        instance: Instance,
        adapter: Adapter,
        device: Device,
        queue: Queue,
        config: AppConfig,
    ) -> Self {
        Self {
            config,
            instance,
            adapter,
            device,
            queue,
            stage: Stage::new(),
        }
    }

    /// Construct a new `Application`.
    ///
    /// # Errors
    ///
    /// - [`Error::AdapterUnavailable`] if no compatible GPU adapter is found.
    /// - [`Error::DeviceRequest`] if device creation fails.
    pub async fn new(config: AppConfig) -> Result<Self, Error> {
        let instance = Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| Error::AdapterUnavailable("no compatible adapter found".to_owned()))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("wisp::Application device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| Error::DeviceRequest(e.to_string()))?;

        let info = adapter.get_info();
        tracing::info!(
            adapter = %info.name,
            backend = ?info.backend,
            device_type = ?info.device_type,
            "wgpu adapter selected"
        );

        Ok(Self {
            config,
            instance,
            adapter,
            device,
            queue,
            stage: Stage::new(),
        })
    }

    /// Borrow the scene graph stage.
    #[must_use]
    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    /// Mutably borrow the scene graph stage.
    pub fn stage_mut(&mut self) -> &mut Stage {
        &mut self.stage
    }

    /// Adapter info (name, backend, device type, driver).
    #[must_use]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// Borrow the wgpu instance.
    #[must_use]
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// Borrow the wgpu adapter.
    #[must_use]
    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    /// Borrow the wgpu device.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Borrow the wgpu queue.
    #[must_use]
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Logical stage width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Logical stage height.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.config.height
    }
}
