//! Setting Satellite up on a device: the launcher is installed and given root, and its setup ([`SetupService` on the
//! device]) is started and followed until it is done. Setup restarts the device on its own and carries on afterwards,
//! so the device is found again by its serial number however it comes back.

use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::adb::Adb;
use crate::device::{self, LAUNCHER};

const PAYLOAD: &str = "/data/local/tmp/satellite";
/// Where what the installer downloaded goes: setup checks these as it checks its own downloads (signatures and
/// checksums), unlike the user's own files in PAYLOAD.
const DOWNLOADED: &str = "/data/local/tmp/satellite/downloaded";
const LOG: &str = "/data/data/dev.emassey0135.satellite.launcher/files/setup.log";
const LAUNCHER_URL: &str = "https://github.com/emassey0135/satellite-launcher/releases/latest/download/satellite-launcher.apk";
// The same as setup downloads on the device (SetupSteps in the launcher).
const LSPOSED_URL: &str = "https://github.com/LSPosed/LSPosed/releases/download/v1.9.2/LSPosed-v1.9.2-7024-zygisk-release.zip";
const LSPOSED_VERSION: u32 = 7024;
const HOOK_URL: &str = "https://github.com/emassey0135/satellite-hook/releases/latest/download/satellite-hook.apk";
const BACKTALK_URL: &str = "https://github.com/trypsynth/backtalk/releases/download/dev/backtalk.apk";

/// The user's choices.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub google_play: bool,
    pub turn_off_dnd: bool,
    /// A language tag such as en-US, or none to leave the language alone.
    pub locale: Option<String>,
    pub launcher_apk: Option<PathBuf>,
    pub hook_apk: Option<PathBuf>,
    pub backtalk_apk: Option<PathBuf>,
    pub lsposed_zip: Option<PathBuf>,
}

/// What the installer tells the user as setup goes.
#[derive(Serialize, Clone)]
pub struct Progress {
    /// "said": something setup said on the device; "info": something the installer is doing; "done"; "stopped".
    pub kind: &'static str,
    pub text: String,
}

fn said(text: impl Into<String>) -> Progress { Progress { kind: "said", text: text.into() } }
fn info(text: impl Into<String>) -> Progress { Progress { kind: "info", text: text.into() } }
fn stopped(text: impl Into<String>) -> Progress { Progress { kind: "stopped", text: text.into() } }

/// Sets Satellite up on the device with serial number `serialno`, telling `report` how it goes.
pub fn run(adb: &Adb, serialno: &str, options: &Options, report: &dyn Fn(Progress)) {
    // Magisk announces each root command on the device, and following setup takes one every few seconds, so its
    // announcements for adb's shell are off while setup runs, and put back as they were afterwards.
    let mut announcements = None;
    let result = start(adb, serialno, options, &mut announcements, report);
    if let (Some(previous), Some(serial)) = (announcements, device::find(adb, serialno)) {
        adb.root(&serial, &format!("magisk --sqlite \"UPDATE policies SET notification={previous} WHERE uid=2000\"\n"));
    }
    if let Err(message) = result {
        report(stopped(message));
    }
}

fn start(adb: &Adb, serialno: &str, options: &Options, announcements: &mut Option<u32>, report: &dyn Fn(Progress))
         -> Result<(), String> {
    let serial = device::find(adb, serialno).ok_or("The Orbit isn't connected any more.")?;

    let files = [
        (&options.hook_apk, "satellite-hook.apk"),
        (&options.backtalk_apk, "backtalk.apk"),
        (&options.lsposed_zip, "lsposed.zip"),
    ];
    if files.iter().any(|(path, _)| path.is_some()) {
        report(info("Copying your files to the Orbit."));
        adb.shell(&serial, &format!("mkdir -p {PAYLOAD}"));
        for (path, name) in files {
            if let Some(path) = path {
                if !adb.push(&serial, path, &format!("{PAYLOAD}/{name}")).ok {
                    return Err(format!("Couldn't copy {} to the Orbit.", path.display()));
                }
            }
        }
    }

    download_for_setup(adb, &serial, options, report);

    let launcher = match &options.launcher_apk {
        Some(path) => path.clone(),
        None => {
            report(info("Downloading the Satellite Launcher."));
            download(LAUNCHER_URL, "satellite-launcher.apk").map_err(|e| format!("Couldn't download the Satellite Launcher: {e}"))?
        }
    };
    report(info("Installing the Satellite Launcher."));
    let installed = adb.on(&serial, &["install", "-r", &launcher.to_string_lossy()]);
    if !installed.text.contains("Success") {
        if installed.text.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") {
            return Err("The Orbit has a copy of the Satellite Launcher signed by someone else. Undo Satellite setup, or \
                        uninstall it, first.".into());
        }
        return Err(format!("Couldn't install the Satellite Launcher: {}", installed.text.trim()));
    }

    // The launcher does its setup with root, which Magisk gives it here, as only the computer can ask.
    let packages = adb.shell(&serial, &format!("pm list packages -U {LAUNCHER}")).text;
    let uid: u32 = packages
        .split("uid:")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .ok_or("Couldn't find the Satellite Launcher after installing it.")?;
    let granted = adb.root(&serial, &format!(
        "magisk --sqlite \"SELECT notification FROM policies WHERE uid=2000\"\n\
         magisk --sqlite \"UPDATE policies SET notification=0 WHERE uid=2000\"\n\
         magisk --sqlite \"REPLACE INTO policies (uid,policy,until,logging,notification) VALUES({uid},2,0,1,0)\"\n\
         magisk --sqlite \"SELECT policy FROM policies WHERE uid={uid}\"\n"));
    *announcements = granted.text.lines()
        .find_map(|l| l.trim().strip_prefix("notification="))
        .and_then(|n| n.parse().ok());
    if !granted.text.contains("policy=2") {
        return Err("Couldn't give the Satellite Launcher root access in Magisk.".into());
    }

    // Only what setup writes from here on is reported.
    let mut seen = log_lines(adb, &serial).map(|l| l.len()).unwrap_or(0);
    let mut start = format!("am start-foreground-service -n {LAUNCHER}/.SetupService --ez google_play {} --ez turn_off_dnd {}",
        options.google_play, options.turn_off_dnd);
    if let Some(locale) = options.locale.as_deref().filter(|l| is_language_tag(l)) {
        start.push_str(&format!(" --es locale {locale}"));
    }
    if !adb.root(&serial, &format!("{start}\n")).ok {
        return Err("Couldn't start setup on the Orbit.".into());
    }
    follow(adb, serialno, serial, &mut seen, report)
}

/// Reports what setup does until it finishes or stops, through its restarts.
fn follow(adb: &Adb, serialno: &str, mut serial: String, seen: &mut usize, report: &dyn Fn(Progress))
          -> Result<(), String> {
    let mut away_since: Option<Instant> = None;
    let mut asked_for_wireless = false;
    let mut stopping = false;
    loop {
        sleep(Duration::from_secs(2));
        let lines = match log_lines(adb, &serial) {
            Some(lines) => lines,
            None => {
                // Restarting: wait for the device to come back, under whatever name.
                let since = *away_since.get_or_insert_with(|| {
                    report(info("The Orbit is restarting. Satellite Setup carries on when it is back."));
                    Instant::now()
                });
                if let Some(found) = device::find(adb, serialno) {
                    serial = found;
                    away_since = None;
                    asked_for_wireless = false;
                    continue;
                }
                if !asked_for_wireless && since.elapsed() > Duration::from_secs(90) {
                    asked_for_wireless = true;
                    report(info("If the Orbit was connected with wireless debugging, turn wireless debugging on again \
                                 on it once it has started, and connect it again here. Setup carries on on the Orbit \
                                 either way."));
                }
                if since.elapsed() > Duration::from_secs(20 * 60) {
                    return Err("The Orbit didn't come back. Setup may still have finished on it; it says so on the \
                                Orbit.".into());
                }
                continue;
            }
        };
        if lines.len() < *seen {
            *seen = 0; // the log was started afresh
        }
        for line in &lines[*seen..] {
            // Lines read "2026-10-15 06:51:31 said: Setting up the device."
            let message = line.splitn(3, ' ').nth(2).unwrap_or("");
            if let Some(text) = message.strip_prefix("said: ") {
                // When setup stops, what it says next is why.
                if stopping || text.starts_with("Setup stopped") {
                    report(stopped(text));
                    return Ok(());
                }
                report(said(text));
            } else if message == "finished" {
                adb.shell(&serial, &format!("rm -rf {PAYLOAD}"));
                report(Progress { kind: "done", text: "Satellite is set up.".into() });
                return Ok(());
            } else if message.starts_with("stopped at ") || message.starts_with("failed at ")
                || message.starts_with("restart failed") {
                stopping = true;
            }
        }
        *seen = lines.len();
    }
}

/// setup.log on the device, or None if the device can't be reached.
fn log_lines(adb: &Adb, serial: &str) -> Option<Vec<String>> {
    let out = adb.root(serial, &format!("cat {LOG} 2>/dev/null; echo end-of-log\n"));
    if !out.text.contains("end-of-log") {
        return None;
    }
    Some(out.text.lines().take_while(|l| *l != "end-of-log").map(str::to_string).collect())
}

/// Downloads on this computer what setup would otherwise download on the Orbit, and copies it there, so that the Orbit
/// needn't be online. Only what the Orbit lacks, and the user didn't give a file for, is downloaded. If a download fails
/// here, the Orbit downloads it itself, if it is online; setup says if it isn't.
fn download_for_setup(adb: &Adb, serial: &str, options: &Options, report: &dyn Fn(Progress)) {
    let lsposed = adb.root(serial, "cat /data/adb/modules/zygisk_lsposed/module.prop 2>/dev/null\n").text;
    let lsposed_installed = lsposed.lines()
        .find_map(|l| l.trim().strip_prefix("versionCode="))
        .and_then(|v| v.trim().parse::<u32>().ok())
        .is_some_and(|v| v >= LSPOSED_VERSION);
    let hook_installed = adb.shell(serial, "pm path dev.emassey0135.satellite.hook").text.contains("package:");
    let backtalk_ready = adb.shell(serial, "cmd package query-activities -a fyi.quin.backtalk.action.ASK_BRAILLE_DISPLAY")
        .text.contains("packageName=fyi.quin.backtalk");
    let wanted = [
        (!lsposed_installed && options.lsposed_zip.is_none(), LSPOSED_URL, "lsposed.zip", "LSPosed"),
        (!hook_installed && options.hook_apk.is_none(), HOOK_URL, "satellite-hook.apk", "the Satellite hook"),
        (!backtalk_ready && options.backtalk_apk.is_none(), BACKTALK_URL, "backtalk.apk", "Backtalk"),
    ];
    let mut got = Vec::new();
    for (needed, url, name, what) in wanted {
        if !needed {
            continue;
        }
        report(info(format!("Downloading {what} on this computer.")));
        match download(url, name) {
            Ok(path) => got.push((path, name)),
            Err(e) => report(info(format!("Couldn't download {what} on this computer ({e}); the Orbit downloads it \
                                           itself, if it is connected to the internet."))),
        }
    }
    if got.is_empty() {
        return;
    }
    report(info("Copying the downloads to the Orbit."));
    adb.shell(serial, &format!("mkdir -p {DOWNLOADED}"));
    for (path, name) in got {
        adb.push(serial, &path, &format!("{DOWNLOADED}/{name}"));
        let _ = std::fs::remove_file(&path);
    }
}

/// Downloads `url` to a file named `name` in this computer's temporary folder.
fn download(url: &str, name: &str) -> Result<PathBuf, String> {
    let mut response = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .and_then(|c| c.get(url).send())
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let path = std::env::temp_dir().join(format!("satellite-{name}"));
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    std::io::copy(&mut response, &mut file).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Whether `tag` is a language tag such as en or en-GB, and so safe to put in a command.
pub fn is_language_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let first = parts.next().unwrap_or("");
    (2..=3).contains(&first.len()) && first.chars().all(|c| c.is_ascii_alphabetic())
        && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Undoes setup: the device's restore script puts back how it was, and the launcher is removed.
pub fn undo(adb: &Adb, serialno: &str, report: &dyn Fn(Progress)) {
    let Some(serial) = device::find(adb, serialno) else {
        report(stopped("The Orbit isn't connected any more."));
        return;
    };
    report(info("Putting the Orbit back as it was before setup."));
    let restored = adb.root(&serial, "sh /data/adb/satellite/restore.sh\n");
    if !restored.text.contains("Restored.") {
        report(stopped(format!("The Orbit couldn't be put back: {}", restored.text.trim())));
        return;
    }
    // Backups made before the restore script did this itself still need it: setting the device up again must make a
    // new backup, of how it is now.
    adb.root(&serial, "rm -rf /data/adb/satellite\n");
    adb.on(&serial, &["uninstall", LAUNCHER]);
    // Orbit's home screen, which the restore made the home app again, in place of the launcher just removed; it also
    // starts what shows Orbit's power options.
    adb.shell(&serial, "am start -a android.intent.action.MAIN -c android.intent.category.HOME");
    report(Progress {
        kind: "done",
        text: "The Orbit is back as it was. Restart it with its power button to finish.".into(),
    });
}

#[cfg(test)]
mod tests {
    use super::is_language_tag;

    #[test]
    fn language_tags() {
        assert!(is_language_tag("en"));
        assert!(is_language_tag("en-GB"));
        assert!(is_language_tag("zh-Hant-TW"));
        assert!(!is_language_tag("e"));
        assert!(!is_language_tag("en GB"));
        assert!(!is_language_tag("en-GB; reboot"));
    }
}
