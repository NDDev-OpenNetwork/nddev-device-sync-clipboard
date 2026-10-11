//! Native point-of-use text clipboard. No Debug, plaintext return through FFI,
//! clipboard history/database copying or implicit image/URI coercion.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use std::sync::Mutex;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use zeroize::Zeroizing;
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum Error {
    #[error("clipboard_unavailable")]
    Unavailable,
    #[error("clipboard_busy")]
    Busy,
    #[error("clipboard_unsupported")]
    Unsupported,
    #[error("clipboard_private")]
    Private,
    #[error("clipboard_too_large")]
    TooLarge,
}
#[derive(Clone)]
pub struct TextClipboard {
    gate: Arc<Semaphore>,
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    clipboard: Arc<Mutex<Option<arboard::Clipboard>>>,
}
impl Default for TextClipboard {
    fn default() -> Self {
        Self {
            gate: Arc::new(Semaphore::new(1)),
            #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
            clipboard: Arc::new(Mutex::new(None)),
        }
    }
}
impl TextClipboard {
    pub async fn write(&self, text: Zeroizing<String>, sensitive: bool) -> Result<(), Error> {
        if text.len() > 8192 || text.contains('\0') {
            return Err(Error::TooLarge);
        }
        let permit = self
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        let clipboard = self.clipboard.clone();
        let work = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
            {
                #[cfg(target_os = "macos")]
                use arboard::SetExtApple;
                #[cfg(target_os = "linux")]
                use arboard::SetExtLinux;
                #[cfg(target_os = "windows")]
                use arboard::SetExtWindows;
                let mut slot = clipboard.lock().map_err(|_| Error::Unavailable)?;
                if slot.is_none() {
                    *slot = Some(arboard::Clipboard::new().map_err(|_| Error::Unavailable)?);
                }
                let set = slot.as_mut().ok_or(Error::Unavailable)?.set();
                // Shared delivery also avoids re-capture and echo. These are
                // native exclusion hints; OS ownership/access remains external.
                #[cfg(target_os = "windows")]
                let set = set.exclude_from_cloud();
                let _ = sensitive;
                set.exclude_from_history()
                    .text(text.as_str())
                    .map_err(|_| Error::Unavailable)
            }
            #[cfg(target_os = "android")]
            {
                android::write(&text, sensitive)
            }
            #[cfg(not(any(
                target_os = "linux",
                target_os = "macos",
                target_os = "windows",
                target_os = "android"
            )))]
            {
                let _ = (text, sensitive);
                Err(Error::Unsupported)
            }
        });
        tokio::time::timeout(Duration::from_secs(3), work)
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(|_| Error::Unavailable)?
    }
    #[cfg(target_os = "android")]
    pub async fn read(&self) -> Result<Zeroizing<String>, Error> {
        let permit = self
            .gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let work = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            android::read()
        });
        tokio::time::timeout(Duration::from_secs(3), work)
            .await
            .map_err(|_| Error::Unavailable)?
            .map_err(|_| Error::Unavailable)?
    }
}
#[cfg(target_os = "android")]
mod android;
