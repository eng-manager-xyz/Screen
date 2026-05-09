//! Crate-level error type.

use thiserror::Error;

/// All errors produced by `wisp`.
///
/// Variants are added as features land. Avoid pre-declaring variants without
/// callers — see `_docs/CONVENTIONS.md` § Error handling.
#[derive(Debug, Error)]
pub enum Error {
    /// No compatible GPU adapter was available.
    #[error("no compatible GPU adapter available: {0}")]
    AdapterUnavailable(String),

    /// Requesting a logical device from the adapter failed.
    #[error("wgpu device request failed: {0}")]
    DeviceRequest(String),

    /// Placeholder for surfaces that haven't been built yet.
    #[error("not yet implemented: {0}")]
    NotImplemented(&'static str),
}
