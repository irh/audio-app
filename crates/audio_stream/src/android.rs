use android_permissions::{PermissionManager, RECORD_AUDIO};
use anyhow::Result;
use jni::{JavaVM, objects::JObject};

#[cfg(target_os = "android")]
pub fn request_recording_permission() -> Result<()> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let manager = PermissionManager::create(vm, activity)?;
    if !manager.check(&RECORD_AUDIO)? {
        manager.request(&[&RECORD_AUDIO])?;
    }

    Ok(())
}
