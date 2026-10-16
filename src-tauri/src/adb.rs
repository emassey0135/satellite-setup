//! Running adb, which comes from Google's platform-tools ([`crate::platform_tools`]).

use std::path::{Path, PathBuf};
use std::process::Command;

/// What a command printed (its output and errors together) and whether it succeeded.
pub struct Output {
    pub ok: bool,
    pub text: String,
}

#[derive(Clone)]
pub struct Adb {
    path: PathBuf,
}

impl Adb {
    pub fn new(path: PathBuf) -> Self {
        Adb { path }
    }

    /// Runs adb with `args`.
    pub fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(&self.path);
        command.args(args);
        hide_window(&mut command);
        match command.output() {
            Ok(out) => {
                let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&out.stderr));
                Output {
                    ok: out.status.success(),
                    text,
                }
            }
            Err(e) => Output {
                ok: false,
                text: e.to_string(),
            },
        }
    }

    /// Runs adb for the device `serial`.
    pub fn on(&self, serial: &str, args: &[&str]) -> Output {
        let mut all = vec!["-s", serial];
        all.extend_from_slice(args);
        self.run(&all)
    }

    /// Runs a shell command on the device, as the shell user. Its words are joined with spaces and read again by the
    /// device's shell, so nothing in it may need quoting.
    pub fn shell(&self, serial: &str, command: &str) -> Output {
        self.on(serial, &["shell", command])
    }

    /// Runs `script` on the device as root. It is copied there and run from the file, so that no quoting is lost on
    /// the way: Magisk's own database has been left unusable by a command whose quotes were.
    pub fn root(&self, serial: &str, script: &str) -> Output {
        const FILE: &str = "/data/local/tmp/satellite-setup.sh";
        let local = std::env::temp_dir().join(format!("satellite-setup-{}.sh", std::process::id()));
        if let Err(e) = std::fs::write(&local, script) {
            return Output {
                ok: false,
                text: e.to_string(),
            };
        }
        let pushed = self.push(serial, &local, FILE);
        let _ = std::fs::remove_file(&local);
        if !pushed.ok {
            return pushed;
        }
        let out = self.on(serial, &["shell", "su", "-c", "sh", FILE]);
        self.shell(serial, &format!("rm -f {FILE}"));
        out
    }

    pub fn push(&self, serial: &str, local: &Path, remote: &str) -> Output {
        self.on(serial, &["push", &local.to_string_lossy(), remote])
    }
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_: &mut Command) {}
