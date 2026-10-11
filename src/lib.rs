//! Native clipboard archive integration. Content stays in the owning provider;
//! native OS text delivery is separate from archive ownership.

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod adapter;
pub mod os;
pub mod view;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use adapter::Clipboard;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use nddev_device_sync_adapter_io::{CancellationToken, IoError, LocalIo, NativeIo};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    #[error(transparent)]
    Io(#[from] IoError),
    #[error("clipboard protocol response is invalid or unsupported")]
    Protocol,
    #[error("clipboard provider refused the request")]
    Provider,
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub trait ReadClipboard {
    fn list(
        &self,
        before: Option<u64>,
        cancellation: CancellationToken,
    ) -> impl std::future::Future<Output = Result<view::Page, Error>> + Send;
    fn fetch(
        &self,
        entry: u64,
        mime: String,
        cancellation: CancellationToken,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, Error>> + Send;
}

/// Canonical compiled-in runtime manifest for this integration.
pub const MANIFEST: &str = include_str!("../contracts/module.json");
