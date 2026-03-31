//! macOS deep-link handler via Apple Events.
//!
//! On macOS, `void://` URL activations are delivered through Apple Events
//! (`kAEGetURL`), not as command-line arguments. This module installs a
//! Carbon Apple Event handler that captures the URL and stores it for the
//! next frame's deep-link processing.

use std::ffi::c_void;
use std::sync::Mutex;

static PENDING_URL: Mutex<Option<String>> = Mutex::new(None);
static EGUI_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

// ── Carbon Apple Event FFI ──────────────────────────────────────────────────

type FourCharCode = u32;

// keyDirectObject = '----'
const KEY_DIRECT_OBJECT: FourCharCode = 0x2D2D2D2D;
// typeUTF8Text = 'utf8'
const TYPE_UTF8_TEXT: FourCharCode = 0x75746638;
// kInternetEventClass = kAEGetURL = 'GURL'
const K_AE_GET_URL: FourCharCode = 0x4755524C;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn AEInstallEventHandler(
        event_class: FourCharCode,
        event_id: FourCharCode,
        handler: extern "C" fn(*const c_void, *mut c_void, isize) -> i16,
        handler_refcon: isize,
        is_sys_handler: u8,
    ) -> i16;

    fn AEGetParamPtr(
        the_apple_event: *const c_void,
        keyword: FourCharCode,
        desired_type: FourCharCode,
        actual_type: *mut FourCharCode,
        data_ptr: *mut u8,
        maximum_size: isize,
        actual_size: *mut isize,
    ) -> i16;
}

// ── Handler ─────────────────────────────────────────────────────────────────

/// Carbon Apple Event callback for `kAEGetURL`.
///
/// Extracts the URL string from the event's `keyDirectObject` parameter
/// and stores it in `PENDING_URL` for the eframe update loop to pick up.
extern "C" fn handle_get_url_event(
    event: *const c_void,
    _reply: *mut c_void,
    _refcon: isize,
) -> i16 {
    if event.is_null() {
        return -1;
    }

    let mut buffer = [0u8; 4096];
    let mut actual_size: isize = 0;
    let mut actual_type: FourCharCode = 0;

    let err = unsafe {
        AEGetParamPtr(
            event,
            KEY_DIRECT_OBJECT,
            TYPE_UTF8_TEXT,
            &mut actual_type,
            buffer.as_mut_ptr(),
            buffer.len() as isize,
            &mut actual_size,
        )
    };

    if err != 0 || actual_size <= 0 {
        return err;
    }

    let len = (actual_size as usize).min(buffer.len());
    let url = String::from_utf8_lossy(&buffer[..len]).to_string();

    if url.starts_with("void://") {
        if let Ok(mut guard) = PENDING_URL.lock() {
            *guard = Some(url);
        }
        // Wake egui so the URL is processed promptly
        if let Ok(guard) = EGUI_CTX.lock() {
            if let Some(ctx) = guard.as_ref() {
                ctx.request_repaint();
            }
        }
    }

    0 // noErr
}

/// Install the Apple Event handler for `kAEGetURL`.
///
/// Must be called early in `main()`, before the run loop starts, so that
/// events arriving during launch are captured.
pub fn install_url_event_handler() {
    let err = unsafe {
        AEInstallEventHandler(
            K_AE_GET_URL, // kInternetEventClass
            K_AE_GET_URL, // kAEGetURL
            handle_get_url_event,
            0,   // refcon
            0u8, // not a system handler
        )
    };
    if err != 0 {
        log::warn!("Failed to install Apple Event URL handler: error {err}");
    }
}

/// Take and clear the pending URL, if any.
pub fn take_pending_url() -> Option<String> {
    PENDING_URL.lock().ok().and_then(|mut g| g.take())
}

/// Store the egui context so the Apple Event callback can trigger repaints.
pub fn set_egui_context(ctx: egui::Context) {
    if let Ok(mut guard) = EGUI_CTX.lock() {
        *guard = Some(ctx);
    }
}
