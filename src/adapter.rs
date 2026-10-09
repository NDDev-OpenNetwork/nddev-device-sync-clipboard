use crate::{
    CancellationToken, Error, IoError, LocalIo, ReadClipboard,
    view::{Entry, Page},
};
use nddev_device_sync_adapter_io::LocalChannel;
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct Clipboard {
    io: LocalIo,
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(tag = "ev", rename_all = "lowercase")]
enum Response {
    Hello {
        v: u8,
    },
    List {
        req: u64,
        items: Vec<Entry>,
        more: bool,
    },
    Blob {
        req: u64,
        mime: String,
        bytes: u64,
    },
    Error {},
}

fn response(channel: &mut LocalChannel) -> Result<Response, Error> {
    let frame = channel.read_frame()?;
    serde_json::from_slice(&frame).map_err(|_| Error::Protocol)
}

fn connect(path: &std::path::Path, cancellation: CancellationToken) -> Result<LocalChannel, Error> {
    let mut channel = LocalChannel::connect(path, Duration::from_secs(5), 65_536, cancellation)?;
    channel.send_frame(br#"{"op":"hello","v":1,"role":"ui","watch":false}"#)?;
    match response(&mut channel)? {
        Response::Hello { v: 1 } => Ok(channel),
        Response::Error {} => Err(Error::Provider),
        _ => Err(Error::Protocol),
    }
}

impl Clipboard {
    pub fn new(io: LocalIo, path: PathBuf) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(IoError::InvalidConfiguration.into());
        }
        Ok(Self { io, path })
    }

    pub fn for_current_user(io: LocalIo) -> Result<Self, Error> {
        let override_root = std::env::var_os("RLDYOUR_CLIPBOARD_HOME");
        #[cfg(target_os = "linux")]
        let native_root = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
        #[cfg(target_os = "macos")]
        let native_root = std::env::var_os("HOME")
            .map(|root| PathBuf::from(root).join("Library/Application Support/rldyour-clipboard"));
        #[cfg(target_os = "windows")]
        let native_root = std::env::var_os("LOCALAPPDATA")
            .map(|root| PathBuf::from(root).join("rldyour-clipboard"));
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let native_root: Option<PathBuf> = None;
        let root = override_root
            .map(PathBuf::from)
            .or(native_root)
            .ok_or(IoError::Unavailable)?;
        Self::new(io, root.join("rldyour-clipboard.sock"))
    }
}

impl ReadClipboard for Clipboard {
    async fn list(
        &self,
        before: Option<u64>,
        cancellation: CancellationToken,
    ) -> Result<Page, Error> {
        if before.is_some_and(|id| id == 0 || id > i64::MAX as u64) {
            return Err(IoError::InvalidConfiguration.into());
        }
        let path = self.path.clone();
        self.io
            .local(cancellation, move |cancellation| {
                Ok((|| {
                    let mut channel = connect(&path, cancellation)?;
                    let mut request = serde_json::json!({"op":"list", "req":1, "limit":50});
                    if let Some(before) = before {
                        request["before"] = before.into();
                    }
                    channel
                        .send_frame(&serde_json::to_vec(&request).map_err(|_| Error::Protocol)?)?;
                    match response(&mut channel)? {
                        Response::List {
                            req: 1,
                            items,
                            more,
                        } if items.len() <= 50 && items.iter().all(Entry::validate) => {
                            Ok(Page { items, more })
                        }
                        Response::Error {} => Err(Error::Provider),
                        _ => Err(Error::Protocol),
                    }
                })())
            })
            .await?
    }

    async fn fetch(
        &self,
        entry: u64,
        mime: String,
        cancellation: CancellationToken,
    ) -> Result<Vec<u8>, Error> {
        if entry == 0 || entry > i64::MAX as u64 || !valid_mime(&mime) {
            return Err(IoError::InvalidConfiguration.into());
        }
        let path = self.path.clone();
        self.io
            .local(cancellation, move |cancellation| {
                Ok((|| {
                    let mut channel = connect(&path, cancellation)?;
                    let request =
                        serde_json::json!({"op":"fetch", "req":1, "entry":entry, "mime":mime});
                    channel
                        .send_frame(&serde_json::to_vec(&request).map_err(|_| Error::Protocol)?)?;
                    match response(&mut channel)? {
                        Response::Blob {
                            req: 1,
                            mime: found,
                            bytes,
                        } if found == mime => {
                            let bytes = usize::try_from(bytes).map_err(|_| IoError::OutputLimit)?;
                            Ok(channel.read_payload(bytes)?)
                        }
                        Response::Error {} => Err(Error::Provider),
                        _ => Err(Error::Protocol),
                    }
                })())
            })
            .await?
    }
}

pub(crate) fn valid_mime(mime: &str) -> bool {
    !mime.is_empty()
        && mime.len() <= 256
        && mime.is_ascii()
        && mime.contains('/')
        && !mime.bytes().any(|byte| byte.is_ascii_control())
}

impl Entry {
    pub(crate) fn validate(&self) -> bool {
        self.id > 0
            && self.id <= i64::MAX as u64
            && !self.mimes.is_empty()
            && self.mimes.len() <= 16
            && self.mimes.iter().all(|mime| valid_mime(mime))
            && matches!(
                self.kind.as_str(),
                "text" | "link" | "color" | "image" | "files"
            )
    }
}
