//! Finding Orbits connected to this computer, and what state each is in.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use crate::adb::Adb;

/// Devices already checked, by adb's name for them. Checking takes root, which Magisk announces on the device each
/// time, so a device is checked only when it appears, when it is connected differently, or when [`forget_checks`] says
/// it has changed.
static CHECKED: Mutex<Option<HashMap<String, Device>>> = Mutex::new(None);

/// Checks every device again the next time they are listed, as after setting one up or undoing setup.
pub fn forget_checks() {
    *CHECKED.lock().unwrap() = None;
}

pub const LAUNCHER: &str = "dev.emassey0135.satellite.launcher";
pub const HOOK: &str = "dev.emassey0135.satellite.hook";
/// Opens Backtalk's question about whether an app may be its braille display, which only a Backtalk with its braille
/// display API has.
pub const ASK_BRAILLE_DISPLAY: &str = "fyi.quin.backtalk.action.ASK_BRAILLE_DISPLAY";

/// A device adb can see.
#[derive(Serialize, Clone)]
pub struct Device {
    /// adb's name for it, which changes with how it is connected.
    pub serial: String,
    /// The device's own serial number, which doesn't.
    pub serialno: String,
    /// "usb" or "wireless".
    pub connection: String,
    /// What adb reports: "device", "unauthorized", "offline", or "no permissions".
    pub state: String,
    pub model: String,
    pub orbit: bool,
    pub root: bool,
    /// Whether Satellite has been set up on it (setup's backup is there).
    pub set_up: bool,
}

/// Every device adb can see, checked as far as its state allows.
pub fn list(adb: &Adb) -> Vec<Device> {
    let out = adb.run(&["devices", "-l"]);
    let mut checked = CHECKED.lock().unwrap();
    let previous = checked.take().unwrap_or_default();
    let mut devices = Vec::new();
    for line in out.text.lines().skip(1) {
        let mut words = line.split_whitespace();
        let Some(serial) = words.next() else { continue };
        let rest: Vec<&str> = words.collect();
        // "no permissions" is two words, followed by a reason in brackets.
        let state = if line.contains("no permissions") {
            "no permissions".to_string()
        } else {
            rest.first().map(|s| s.to_string()).unwrap_or_default()
        };
        let model = rest
            .iter()
            .find_map(|w| w.strip_prefix("model:"))
            .unwrap_or("")
            .replace('_', " ");
        let connection =
            if rest.iter().any(|w| w.starts_with("usb:")) || !serial.contains(':') && !serial.contains("._adb-tls") {
                "usb"
            } else {
                "wireless"
            };
        let mut device = Device {
            serial: serial.to_string(),
            serialno: String::new(),
            connection: connection.to_string(),
            state,
            model,
            orbit: false,
            root: false,
            set_up: false,
        };
        if device.state == "device" {
            match previous.get(serial) {
                Some(known) => {
                    device.serialno = known.serialno.clone();
                    device.orbit = known.orbit;
                    device.root = known.root;
                    device.set_up = known.set_up;
                }
                None => check(adb, &mut device),
            }
        }
        devices.push(device);
    }
    *checked = Some(
        devices
            .iter()
            .filter(|d| d.state == "device")
            .map(|d| (d.serial.clone(), d.clone()))
            .collect(),
    );
    devices
}

fn check(adb: &Adb, device: &mut Device) {
    let serial = device.serial.clone();
    device.serialno = adb.shell(&serial, "getprop ro.serialno").text.trim().to_string();
    device.orbit = adb
        .shell(&serial, "pm path com.orbit.screenreader")
        .text
        .contains("package:");
    let root = adb.root(&serial, "id; [ -f /data/adb/satellite/restore.sh ] && echo set-up\n");
    device.root = root.text.contains("uid=0");
    device.set_up = root.text.contains("set-up");
}

/// adb's current name for the device with this serial number, however it is connected now.
pub fn find(adb: &Adb, serialno: &str) -> Option<String> {
    let out = adb.run(&["devices"]);
    for line in out.text.lines().skip(1) {
        let mut words = line.split_whitespace();
        let (Some(serial), Some("device")) = (words.next(), words.next()) else {
            continue;
        };
        if adb.shell(serial, "getprop ro.serialno").text.trim() == serialno {
            return Some(serial.to_string());
        }
    }
    None
}
