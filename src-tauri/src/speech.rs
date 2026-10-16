//! Speaking through speech-dispatcher, on Linux while Orca is running. Orca doesn't hear WebKitGTK's live regions,
//! which the window uses everywhere else (tested with polite, assertive, atomic, alert and status regions), but it
//! speaks through speech-dispatcher, as these messages then do. One connection is kept for the installer's run, so the
//! messages arrive in order from one client; if it fails, the window falls back to its live region, and the next
//! message connects again.

#[cfg(target_os = "linux")]
mod imp {
    use std::os::unix::net::UnixStream;
    use std::process::Command;
    use std::sync::Mutex;
    use std::time::Duration;

    use ssip_client_async::{fifo, Client, ClientName, ClientResult, Priority, OK_PRIORITY_SET};

    static CLIENT: Mutex<Option<Client<UnixStream>>> = Mutex::new(None);

    /// Says `text`, and returns whether it did: only while Orca is running, so that nobody else hears it.
    pub fn say(text: &str) -> bool {
        if !Command::new("pgrep").args(["-x", "orca"]).output().is_ok_and(|o| o.status.success()) {
            return false;
        }
        let mut client = CLIENT.lock().unwrap();
        if client.is_none() {
            *client = connect().ok();
        }
        let Some(connected) = client.as_mut() else { return false };
        // Lines are what the protocol sends, and a line of just "." would end the message early.
        let line = text.replace(['\r', '\n'], " ");
        let said = connected.speak().and_then(|c| c.check_receiving_data()).and_then(|c| c.send_line(&line))
            .and_then(|c| c.receive_message_id());
        if said.is_err() {
            *client = None;
        }
        said.is_ok()
    }

    /// Connects as speech-dispatcher's own clients (such as spd-say) do: to SPEECHD_ADDRESS if it names a Unix socket
    /// (as when the system shares one speech-dispatcher, such as with the login screen), else to the default socket in
    /// the user's runtime folder, starting speech-dispatcher there if it isn't running.
    fn connect() -> ClientResult<Client<UnixStream>> {
        let mut builder = fifo::synchronous::Builder::new();
        match std::env::var("SPEECHD_ADDRESS").ok().and_then(|a| a.strip_prefix("unix_socket:").map(str::to_string)) {
            Some(path) => {
                builder.path(path);
            }
            None => {
                builder.with_spawn()?;
            }
        }
        let mut client = builder.timeout(Duration::from_secs(5)).build()?;
        client
            .set_client_name(ClientName::new("satellite-setup", "progress"))?
            .check_client_name_set()?
            // "message" queues behind what is being said, rather than cutting it off or being cut off.
            .set_priority(Priority::Message)?
            .check_status(OK_PRIORITY_SET)?;
        Ok(client)
    }
}

#[cfg(target_os = "linux")]
pub use imp::say;

#[cfg(not(target_os = "linux"))]
pub fn say(_: &str) -> bool {
    false
}
