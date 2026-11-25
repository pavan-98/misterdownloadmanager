//! `libmdm.so`: Mister Download Manager on Android. The Android project (android/) runs it in a
//! GameActivity (`MainActivity`), whose native thread starts here, in [`android_main`]: the app's
//! folders are set, HTTPS is given Android's certificate checks, the helper programs the APK
//! carries are unpacked, and then the app runs as on the desktop (`mdm_app::android::run`).

#![cfg(target_os = "android")]
// `android_main` is an exported C function and the activity's folders come through JNI: this
// crate is Android's FFI, as win32.rs is Windows'.
#![allow(unsafe_code)]

use std::path::PathBuf;

use egui_winit::winit::platform::android::activity::AndroidApp;
use jni::objects::{JObject, JString};
use jni::{Env, JavaVM, jni_sig, jni_str};

/// The activity's native thread (android-activity calls it once the activity is created): the
/// app until the activity ends.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("mdm"),
    );
    let folders = match folders(&app) {
        Ok(f) => f,
        Err(e) => {
            log::error!("the activity's folders: {e}");
            return;
        }
    };
    // The list, settings and logs: the app's private files.
    mdm_ipc::set_data_dir(folders.data.clone());
    // yt-dlp's Python, ffmpeg, QuickJS and 7-Zip from the APK, unpacked on a thread of their own.
    mdm_tools::android::init(&folders.native_lib, &folders.data, &folders.cache);
    if let Err(e) = mdm_app::android::run(app) {
        log::error!("the app stopped: {e}");
    }
}

/// Where the app keeps and finds things.
struct Folders {
    /// `getFilesDir()/mdm`: the list, settings, logs, the helpers once unpacked.
    data: PathBuf,
    /// `getCacheDir()`: temporary files.
    cache: PathBuf,
    /// `getApplicationInfo().nativeLibraryDir`: the programs the APK carries.
    native_lib: PathBuf,
}

/// The activity's folders, and HTTPS given Android's own certificate checks (and the user's
/// CAs) while the activity is at hand.
fn folders(app: &AndroidApp) -> Result<Folders, String> {
    let data = app
        .internal_data_path()
        .ok_or("no internal data folder")?
        .join("mdm");
    // SAFETY: android-activity's JavaVM and its activity, a global reference that outlives this.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<Result<Folders, String>> {
        // SAFETY: the activity's global reference, valid while the activity lives; not deleted.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let native_lib = native_library_dir(env, &activity)?;
        let cache = cache_dir(env, &activity)?;
        if let Err(e) = rustls_platform_verifier::android::init_with_env(env, activity) {
            return Ok(Err(format!("certificate checks: {e}")));
        }
        Ok(Ok(Folders {
            data,
            cache,
            native_lib,
        }))
    })
    .map_err(|e| e.to_string())?
}

/// `context.getApplicationInfo().nativeLibraryDir`.
fn native_library_dir(env: &mut Env, context: &JObject) -> jni::errors::Result<PathBuf> {
    let info = env
        .call_method(
            context,
            jni_str!("getApplicationInfo"),
            jni_sig!("()Landroid/content/pm/ApplicationInfo;"),
            &[],
        )?
        .l()?;
    let dir = env
        .get_field(
            &info,
            jni_str!("nativeLibraryDir"),
            jni_sig!("Ljava/lang/String;"),
        )?
        .l()?;
    let dir = env.cast_local::<JString>(dir)?;
    Ok(PathBuf::from(dir.try_to_string(env)?))
}

/// `context.getCacheDir().getAbsolutePath()`.
fn cache_dir(env: &mut Env, context: &JObject) -> jni::errors::Result<PathBuf> {
    let dir = env
        .call_method(
            context,
            jni_str!("getCacheDir"),
            jni_sig!("()Ljava/io/File;"),
            &[],
        )?
        .l()?;
    let path = env
        .call_method(
            &dir,
            jni_str!("getAbsolutePath"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let path = env.cast_local::<JString>(path)?;
    Ok(PathBuf::from(path.try_to_string(env)?))
}

/// `MainActivity.nativeBack()`: the system's back gesture or button reached the activity's back
/// dispatcher (Android 13 and later send no key for it); the app closes the window in front.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_misterdownloadmanager_android_MainActivity_nativeBack(
    _env: *mut jni::sys::JNIEnv,
    _this: jni::sys::jobject,
) {
    mdm_app::android::back_pressed();
}

/// `MainActivity.nativeEnding()`: the activity is being destroyed. Its native side then waits for
/// `android_main` to return, so the app closes first (`mdm_app::android::activity_ending`).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_misterdownloadmanager_android_MainActivity_nativeEnding(
    _env: *mut jni::sys::JNIEnv,
    _this: jni::sys::jobject,
) {
    mdm_app::android::activity_ending();
}
