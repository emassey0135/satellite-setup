//! Satellite Setup: sets Satellite up on an Orbit Speak or Orbit Player connected to this computer. The window (ui/)
//! calls the commands here, and setup's progress comes back to it as "progress" events.

mod adb;
mod device;
mod platform_tools;
mod setup;

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, State};

use adb::Adb;

/// adb, once platform-tools are ready.
#[derive(Default)]
struct Tools(Mutex<Option<Adb>>);

impl Tools {
    fn adb(&self) -> Result<Adb, String> {
        self.0.lock().unwrap().clone().ok_or_else(|| "Android's tools aren't ready yet.".to_string())
    }
}

/// Gets Android's tools ready, downloading them the first time.
#[tauri::command]
async fn prepare(app: AppHandle, tools: State<'_, Tools>) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = tauri::async_runtime::spawn_blocking(move || platform_tools::ensure(&dir))
        .await
        .map_err(|e| e.to_string())??;
    let adb = Adb::new(path);
    let started = adb.clone();
    tauri::async_runtime::spawn_blocking(move || started.run(&["start-server"])).await.map_err(|e| e.to_string())?;
    *tools.0.lock().unwrap() = Some(adb);
    Ok(())
}

#[tauri::command]
async fn devices(tools: State<'_, Tools>) -> Result<Vec<device::Device>, String> {
    let adb = tools.adb()?;
    tauri::async_runtime::spawn_blocking(move || device::list(&adb)).await.map_err(|e| e.to_string())
}

/// Pairs with a device's wireless debugging: the address and code it shows under "Pair device with pairing code".
#[tauri::command]
async fn pair(tools: State<'_, Tools>, address: String, code: String) -> Result<String, String> {
    let adb = tools.adb()?;
    let out = tauri::async_runtime::spawn_blocking(move || adb.run(&["pair", address.trim(), code.trim()]))
        .await
        .map_err(|e| e.to_string())?;
    if out.text.contains("Successfully paired") { Ok("Paired.".into()) } else { Err(out.text.trim().to_string()) }
}

/// Connects to a device's wireless debugging, at the address it shows under Wireless debugging.
#[tauri::command]
async fn connect(tools: State<'_, Tools>, address: String) -> Result<String, String> {
    let adb = tools.adb()?;
    let out = tauri::async_runtime::spawn_blocking(move || adb.run(&["connect", address.trim()]))
        .await
        .map_err(|e| e.to_string())?;
    if out.text.contains("connected to") { Ok("Connected.".into()) } else { Err(out.text.trim().to_string()) }
}

/// Sets Satellite up on the device with serial number `serialno`; progress comes as "progress" events.
#[tauri::command]
fn set_up(app: AppHandle, tools: State<'_, Tools>, serialno: String, options: setup::Options) -> Result<(), String> {
    let adb = tools.adb()?;
    std::thread::spawn(move || setup::run(&adb, &serialno, &options, &|p| { let _ = app.emit("progress", p); }));
    Ok(())
}

/// Undoes setup on the device with serial number `serialno`; progress comes as "progress" events.
#[tauri::command]
fn undo(app: AppHandle, tools: State<'_, Tools>, serialno: String) -> Result<(), String> {
    let adb = tools.adb()?;
    std::thread::spawn(move || setup::undo(&adb, &serialno, &|p| { let _ = app.emit("progress", p); }));
    Ok(())
}

/// On Linux, lets the user's account use the Orbit over USB: a rule for MediaTek's USB vendor ID, which the Orbits
/// use, added as root (the system asks for the administrator's password).
#[tauri::command]
async fn allow_usb() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let script = "printf '%s\\n' 'SUBSYSTEM==\"usb\", ATTR{idVendor}==\"0e8d\", MODE=\"0660\", TAG+=\"uaccess\"' \
                      > /etc/udev/rules.d/51-satellite-orbit.rules && udevadm control --reload-rules && udevadm trigger";
        let out = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("pkexec").args(["sh", "-c", script]).output()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok("This computer can use the Orbit over USB now. Unplug it and plug it in again.".into())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "linux"))]
    Err("Only Linux needs this.".into())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Tools::default())
        .invoke_handler(tauri::generate_handler![prepare, devices, pair, connect, set_up, undo, allow_usb])
        .run(tauri::generate_context!())
        .expect("error while running Satellite Setup");
}
