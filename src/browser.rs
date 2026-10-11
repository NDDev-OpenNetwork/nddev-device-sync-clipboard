//! Explicit browser clipboard delivery. Reserve synchronously inside the user
//! gesture; only the Rust secret consumer can complete the pending item.
use crate::os::Error;
use wasm_bindgen::prelude::*;
use zeroize::Zeroizing;

#[wasm_bindgen(module = "/src/browser_clipboard.js")]
extern "C" {
    #[wasm_bindgen(js_name=clipboardSupported)]
    pub fn supported() -> bool;
    #[wasm_bindgen(catch,js_name=reserveClipboard)]
    fn reserve() -> Result<String, JsValue>;
    #[wasm_bindgen(catch,js_name=completeClipboard)]
    fn complete(id: &str, text: &str) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(js_name=cancelClipboard)]
    fn cancel(id: &str);
}
fn failure(error: JsValue) -> Error {
    use wasm_bindgen::JsCast;
    match error
        .dyn_ref::<js_sys::Error>()
        .map(|e| String::from(e.message()))
        .as_deref()
    {
        Some("clipboard_busy") => Error::Busy,
        Some("clipboard_permission_denied") => Error::PermissionDenied,
        Some("clipboard_unsupported") => Error::Unsupported,
        _ => Error::Unavailable,
    }
}
pub fn reserve_write() -> Result<String, Error> {
    reserve().map_err(failure)
}
pub fn cancel_write(id: &str) {
    cancel(id);
}
struct Pending(String);
impl Drop for Pending {
    fn drop(&mut self) {
        cancel(&self.0);
    }
}
pub async fn write_reserved(id: String, text: Zeroizing<String>) -> Result<(), Error> {
    let pending = Pending(id);
    if text.len() > 8192 || text.contains('\0') {
        return Err(Error::TooLarge);
    }
    wasm_bindgen_futures::JsFuture::from(complete(&pending.0, &text).map_err(failure)?)
        .await
        .map_err(failure)?;
    Ok(())
}
