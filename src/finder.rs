//! Files opened from Finder (double-click, "Open With") arrive as an AppKit event, not as
//! command-line arguments. A tiny app delegate collects them for the app to load.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static OPENED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static CTX: OnceLock<eframe::egui::Context> = OnceLock::new();

/// Lets a file opened while the app is idle wake it up.
pub fn set_ctx(ctx: &eframe::egui::Context) {
    let _ = CTX.set(ctx.clone());
}

/// Files Finder asked us to open since the last call.
pub fn take() -> Vec<PathBuf> {
    std::mem::take(&mut *OPENED.lock().unwrap())
}

#[cfg(target_os = "macos")]
pub use mac::install;

#[cfg(not(target_os = "macos"))]
pub fn install() {}

#[cfg(target_os = "macos")]
mod mac {
    use super::{CTX, OPENED};
    use objc2::runtime::{AnyClass, AnyObject, Sel};
    use objc2::sel;
    use objc2_app_kit::NSApplicationWillFinishLaunchingNotification;
    use objc2_foundation::{NSArray, NSNotification, NSNotificationCenter, NSURL};
    use std::path::PathBuf;
    use std::ptr::NonNull;

    extern "C" fn open_urls(_this: *mut AnyObject, _cmd: Sel, _app: *mut AnyObject, urls: *mut NSArray<NSURL>) {
        let Some(urls) = (unsafe { urls.as_ref() }) else { return };
        let paths: Vec<PathBuf> = urls.iter().filter_map(|u| unsafe { u.path() }).map(|p| PathBuf::from(p.to_string())).collect();
        OPENED.lock().unwrap().extend(paths);
        if let Some(c) = CTX.get() {
            c.request_repaint();
        }
    }

    /// Call before eframe starts. winit owns the app delegate (replacing it crashes winit), so
    /// `application:openURLs:` is added to winit's delegate class instead, at "will finish
    /// launching", just before AppKit hands over the files to open.
    pub fn install() {
        let block = block2::RcBlock::new(|_: NonNull<NSNotification>| unsafe {
            // ponytail: relies on winit's class name; if it changes, Finder opens just stop working
            let Some(cls) = AnyClass::get("WinitApplicationDelegate") else { return };
            let imp: unsafe extern "C" fn() = std::mem::transmute(open_urls as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, *mut NSArray<NSURL>));
            objc2::ffi::class_addMethod(cls as *const AnyClass as *mut objc2::ffi::objc_class, sel!(application:openURLs:).as_ptr(), Some(imp), c"v@:@@".as_ptr());
        });
        unsafe {
            let token = NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(Some(NSApplicationWillFinishLaunchingNotification), None, None, &block);
            std::mem::forget(token);
        }
    }
}
