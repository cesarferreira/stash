//! Background agent: no Dock icon + optional LaunchAgent at login.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

pub const LAUNCH_AGENT_LABEL: &str = "dev.stash.agent";

/// Hide from Dock / Cmd-Tab as a menu-bar-style accessory process.
pub fn become_accessory() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}

pub fn is_agent_launch() -> bool {
    std::env::args().any(|a| a == "--agent")
}

fn launch_agents_dir() -> PathBuf {
    home_dir().join("Library/LaunchAgents")
}

fn plist_path() -> PathBuf {
    launch_agents_dir().join(format!("{LAUNCH_AGENT_LABEL}.plist"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn plist_body(program: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LAUNCH_AGENT_LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{program}</string>
		<string>--agent</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<dict>
		<key>SuccessfulExit</key>
		<false/>
	</dict>
	<key>ProcessType</key>
	<string>Interactive</string>
	<key>ThrottleInterval</key>
	<integer>5</integer>
</dict>
</plist>
"#
    )
}

fn uid() -> String {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "501".into())
}

fn launchctl_bootout(plist: &PathBuf) {
    let domain = format!("gui/{}", uid());
    let _ = Command::new("launchctl")
        .args(["bootout", &domain])
        .arg(plist)
        .status();
    let _ = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(plist)
        .status();
}

fn launchctl_bootstrap(plist: &PathBuf) -> Result<(), String> {
    let domain = format!("gui/{}", uid());
    let status = Command::new("launchctl")
        .args(["bootstrap", &domain])
        .arg(plist)
        .status()
        .map_err(|e| format!("launchctl bootstrap: {e}"))?;
    if status.success() {
        return Ok(());
    }
    // Fallback for older macOS / already-loaded edge cases.
    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(plist)
        .status()
        .map_err(|e| format!("launchctl load: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to load LaunchAgent {}", plist.display()))
    }
}

/// Install or remove `~/Library/LaunchAgents/dev.stash.agent.plist` so stash
/// starts at login and keeps capturing after reboot.
pub fn sync_launch_agent(enabled: bool) -> Result<(), String> {
    let plist = plist_path();
    if !enabled {
        if plist.exists() {
            launchctl_bootout(&plist);
            let _ = fs::remove_file(&plist);
        }
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let exe = exe
        .canonicalize()
        .unwrap_or(exe)
        .to_string_lossy()
        .into_owned();

    // Don't wire `cargo run` debug builds into login items.
    if exe.contains("/target/debug/") {
        return Ok(());
    }

    let dir = launch_agents_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    let body = plist_body(&exe);
    let previous = fs::read_to_string(&plist).unwrap_or_default();
    if previous == body && plist.exists() {
        return Ok(());
    }

    launchctl_bootout(&plist);
    fs::write(&plist, &body).map_err(|e| format!("write {}: {e}", plist.display()))?;
    launchctl_bootstrap(&plist)?;
    eprintln!(
        "stash: LaunchAgent installed ({}) — starts at login",
        plist.display()
    );
    Ok(())
}
