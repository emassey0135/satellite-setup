//! Google's platform-tools, which include adb, downloaded for this computer the first time the installer runs, so that
//! nothing else has to be installed and it is always Google's current version. Google builds them for Linux only on
//! x86_64, so elsewhere on Linux (such as ARM) the system's own adb is used, from the distribution's package.

use std::io::Read;
use std::path::{Path, PathBuf};

fn url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
    } else if cfg!(target_os = "macos") {
        "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
    } else {
        "https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
    }
}

fn offline(e: impl std::fmt::Display) -> String {
    format!(
        "Couldn't download Android's tools ({e}). Check that this computer is connected to the internet, then open \
             Satellite Setup again."
    )
}

/// The first `adb` on the PATH.
fn system_adb() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join("adb"))
        .find(|p| p.is_file())
}

fn adb_in(dir: &Path) -> PathBuf {
    dir.join("platform-tools")
        .join(if cfg!(windows) { "adb.exe" } else { "adb" })
}

/// adb, from platform-tools in `dir`, downloaded first if they aren't there yet.
pub fn ensure(dir: &Path) -> Result<PathBuf, String> {
    if cfg!(target_os = "linux") && !cfg!(target_arch = "x86_64") {
        return system_adb().ok_or_else(|| {
            "Google doesn't make Android's tools for this kind of computer, so Satellite Setup needs your Linux \
             distribution's adb. Install it (on Arch Linux, the android-tools package; on Debian and Ubuntu, the adb \
             package), then open Satellite Setup again."
                .to_string()
        });
    }
    let adb = adb_in(dir);
    if adb.exists() {
        return Ok(adb);
    }
    let mut bytes = Vec::new();
    reqwest::blocking::get(url())
        .and_then(|r| r.error_for_status())
        .map_err(offline)?
        .read_to_end(&mut bytes)
        .map_err(offline)?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    // Unpacked beside the final place first, so that a half-unpacked copy is never taken for a whole one.
    let partial = dir.join("platform-tools.partial");
    let _ = std::fs::remove_dir_all(&partial);
    zip.extract(&partial)
        .map_err(|e| format!("Couldn't unpack Android's tools: {e}"))?;
    let _ = std::fs::remove_dir_all(dir.join("platform-tools"));
    std::fs::rename(partial.join("platform-tools"), dir.join("platform-tools")).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&partial);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o755));
    }
    Ok(adb)
}
