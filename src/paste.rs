//! Restore previous app focus and synthesize ⌘V after a copy-pasta selection.

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static PREVIOUS_PID: Mutex<Option<i32>> = Mutex::new(None);

/// Remember the frontmost app if it isn't copy-pasta itself (call before activating the popup).
pub fn remember_frontmost_excluding_self() {
    let Some(app) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
        return;
    };
    let pid = app.processIdentifier();
    if pid == std::process::id() as i32 {
        return;
    }
    if let Ok(mut slot) = PREVIOUS_PID.lock() {
        *slot = Some(pid);
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
