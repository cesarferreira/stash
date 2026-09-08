//! Restore previous app focus and synthesize ⌘V after a copy-pasta selection.

use accessibility_sys::{
    kAXFocusedApplicationAttribute, pid_t, AXUIElementCopyAttributeValue,
    AXUIElementCreateSystemWide, AXUIElementGetPid, AXUIElementRef,
};
use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static PREVIOUS_PID: Mutex<Option<i32>> = Mutex::new(None);

/// Remember the app that owns the actual focused UI element if it isn't
/// copy-pasta itself (call before activating the popup).
///
/// `NSWorkspace.frontmostApplication` only reports the frontmost *regular*
/// app — floating "quick capture" panels (ChatGPT's Ask overlay, Spotlight,
/// Raycast, …) grab keyboard focus without making their owning app frontmost,
/// so that heuristic silently records whatever normal window sits underneath.
/// The Accessibility API's system-wide focused-application attribute follows
/// real keyboard focus instead, so it still finds the panel's owner.
pub fn remember_frontmost_excluding_self() {
    let self_pid = std::process::id() as i32;
    let pid = focused_application_pid().or_else(|| {
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| app.processIdentifier())
    });
    let Some(pid) = pid else {
        return;
    };
    if pid == self_pid {
        return;
    }
    if let Ok(mut slot) = PREVIOUS_PID.lock() {
        *slot = Some(pid);
    }
}

/// pid of the process that owns the system-wide focused UI element, via the
/// Accessibility API. Requires the Accessibility permission we already need
/// for keystroke synthesis; returns `None` if it's not granted or the call
/// fails for any reason.
fn focused_application_pid() -> Option<i32> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }
        let attr = CFString::new(kAXFocusedApplicationAttribute);
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(system_wide, attr.as_concrete_TypeRef(), &mut value);
        CFRelease(system_wide as CFTypeRef);
        if err != 0 || value.is_null() {
            return None;
        }
        let app_element = value as AXUIElementRef;
        let mut pid: pid_t = 0;
        let pid_err = AXUIElementGetPid(app_element, &mut pid);
        CFRelease(value);
        if pid_err != 0 {
            return None;
        }
        Some(pid)
    }
}

fn previous_pid() -> Option<i32> {
    PREVIOUS_PID.lock().ok().and_then(|g| *g)
}

/// Bring the remembered app forward (best-effort) then send ⌘V.
pub fn activate_previous_and_paste() {
    if let Some(pid) = previous_pid() {
        activate_pid(pid);
        // ⌘-key equivalents are only routed to the frontmost/key app, so wait for
        // activation to actually land before sending the keystroke — a fixed
        // sleep here was racy and could fire before the target app regained
        // focus, dropping the paste (or misdirecting it) intermittently.
        wait_until_frontmost(pid, Duration::from_millis(500));
    }
    if let Err(err) = synthesize_command_v() {
        eprintln!("copy-pasta: paste keystroke failed: {err}");
        eprintln!(
            "copy-pasta: grant Accessibility to copy-pasta in System Settings → Privacy & Security → Accessibility"
        );
    }
}

fn activate_pid(pid: i32) {
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return;
    };
    let _ = app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
}

fn wait_until_frontmost(pid: i32, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let is_front = NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .is_some_and(|app| app.processIdentifier() == pid);
        if is_front {
            return;
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}

fn synthesize_command_v() -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|()| "CGEventSource unavailable".to_string())?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), KeyCode::ANSI_V, true)
        .map_err(|()| "keydown failed".to_string())?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);

    let key_up = CGEvent::new_keyboard_event(source, KeyCode::ANSI_V, false)
        .map_err(|()| "keyup failed".to_string())?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);

    // Prefer targeting the previous app when we know its pid.
    if let Some(pid) = previous_pid() {
        key_down.post_to_pid(pid);
        key_up.post_to_pid(pid);
    } else {
        key_down.post(CGEventTapLocation::HID);
        key_up.post(CGEventTapLocation::HID);
    }
    Ok(())
}
