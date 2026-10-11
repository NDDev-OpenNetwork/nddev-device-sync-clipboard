use super::Error;
use jni::{
    JNIEnv,
    objects::{JObject, JString, JValue},
};
use zeroize::Zeroizing;
fn failure<T>(value: Result<T, jni::errors::Error>) -> Result<T, Error> {
    value.map_err(|_| Error::Unavailable)
}
fn manager<'local>(
    env: &mut JNIEnv<'local>,
    app: &JObject<'static>,
) -> Result<JObject<'local>, Error> {
    let name = failure(env.new_string("clipboard"))?;
    failure(env.call_method(
        app,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&name)],
    ))?
    .l()
    .map_err(|_| Error::Unavailable)
}
pub(super) fn write(text: &str, sensitive: bool) -> Result<(), Error> {
    nddev_device_sync_accounts::android::with_application_context(|env, app| {
        Ok((|| {
            let manager = manager(env, app)?;
            let label = failure(env.new_string("NDS"))?;
            let text = failure(env.new_string(text))?;
            let clip = failure(env.call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&text)],
            ))?
            .l()
            .map_err(|_| Error::Unavailable)?;
            let description = failure(env.call_method(
                &clip,
                "getDescription",
                "()Landroid/content/ClipDescription;",
                &[],
            ))?
            .l()
            .map_err(|_| Error::Unavailable)?;
            let extras = failure(env.new_object("android/os/PersistableBundle", "()V", &[]))?;
            // Both sensitive values and delivered remote values must not re-enter
            // shared capture/history. Android also suppresses secret previews.
            let _ = sensitive;
            let key = failure(env.new_string("android.content.extra.IS_SENSITIVE"))?;
            failure(env.call_method(
                &extras,
                "putBoolean",
                "(Ljava/lang/String;Z)V",
                &[JValue::Object(&key), JValue::Bool(1)],
            ))?;
            failure(env.call_method(
                description,
                "setExtras",
                "(Landroid/os/PersistableBundle;)V",
                &[JValue::Object(&extras)],
            ))?;
            failure(env.call_method(
                manager,
                "setPrimaryClip",
                "(Landroid/content/ClipData;)V",
                &[JValue::Object(&clip)],
            ))?;
            Ok(())
        })())
    })
    .map_err(|_| Error::Unavailable)?
}
pub(super) fn read() -> Result<Zeroizing<String>, Error> {
    nddev_device_sync_accounts::android::with_application_context(|env, app| {
        Ok((|| {
            let manager = manager(env, app)?;
            // Read one ClipData snapshot: separately fetching its description
            // and payload could admit a newly replaced sensitive clipboard.
            let clip = failure(env.call_method(
                manager,
                "getPrimaryClip",
                "()Landroid/content/ClipData;",
                &[],
            ))?
            .l()
            .map_err(|_| Error::Unavailable)?;
            if clip.is_null() {
                return Err(Error::Unavailable);
            }
            let description = failure(env.call_method(
                &clip,
                "getDescription",
                "()Landroid/content/ClipDescription;",
                &[],
            ))?
            .l()
            .map_err(|_| Error::Unavailable)?;
            if description.is_null() {
                return Err(Error::Unavailable);
            }
            let mime = failure(env.new_string("text/plain"))?;
            if !failure(env.call_method(
                &description,
                "hasMimeType",
                "(Ljava/lang/String;)Z",
                &[JValue::Object(&mime)],
            ))?
            .z()
            .map_err(|_| Error::Unavailable)?
            {
                return Err(Error::Unsupported);
            }
            let extras = failure(env.call_method(
                description,
                "getExtras",
                "()Landroid/os/PersistableBundle;",
                &[],
            ))?
            .l()
            .map_err(|_| Error::Unavailable)?;
            if !extras.is_null() {
                let key = failure(env.new_string("android.content.extra.IS_SENSITIVE"))?;
                if failure(env.call_method(
                    extras,
                    "getBoolean",
                    "(Ljava/lang/String;Z)Z",
                    &[JValue::Object(&key), JValue::Bool(0)],
                ))?
                .z()
                .map_err(|_| Error::Unavailable)?
                {
                    return Err(Error::Private);
                }
            }
            if failure(env.call_method(&clip, "getItemCount", "()I", &[]))?
                .i()
                .map_err(|_| Error::Unavailable)?
                != 1
            {
                return Err(Error::Unsupported);
            }
            let item = failure(env.call_method(
                clip,
                "getItemAt",
                "(I)Landroid/content/ClipData$Item;",
                &[JValue::Int(0)],
            ))?
            .l()
            .map_err(|_| Error::Unavailable)?;
            let text =
                failure(env.call_method(item, "getText", "()Ljava/lang/CharSequence;", &[]))?
                    .l()
                    .map_err(|_| Error::Unavailable)?;
            if text.is_null() {
                return Err(Error::Unsupported);
            }
            let length = failure(env.call_method(&text, "length", "()I", &[]))?
                .i()
                .map_err(|_| Error::Unavailable)?;
            if !(0..=8192).contains(&length) {
                return Err(Error::TooLarge);
            }
            let text: JString =
                failure(env.call_method(text, "toString", "()Ljava/lang/String;", &[]))?
                    .l()
                    .map_err(|_| Error::Unavailable)?
                    .into();
            let value: String = failure(env.get_string(&text))?.into();
            if value.len() > 8192 || value.contains('\0') {
                return Err(Error::TooLarge);
            }
            Ok(Zeroizing::new(value))
        })())
    })
    .map_err(|_| Error::Unavailable)?
}
