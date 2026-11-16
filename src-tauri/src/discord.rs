//! Discord status ("Rich Presence"): talks to the Discord app on this computer, no internet
//! service involved. A background thread owns the connection, reconnects when Discord is started
//! later, and quietly does nothing when Discord isn't running.

use discord_rich_presence::activity::{Activity, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::Deserialize;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::State;

use crate::state::AppState;

/// The AniLauncher application on Discord's developer portal. A build can override it with the
/// DISCORD_APP_ID variable, and the player in the settings.
fn default_app_id() -> &'static str {
    match option_env!("ANILAUNCHER_DISCORD_ID") {
        Some(id) if !id.trim().is_empty() => id,
        _ => "1571852560866873454",
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    pub details: String,
    pub state: String,
    /// unix seconds the timer counts from
    pub start: Option<i64>,
    pub large_text: String,
}

enum Msg {
    Set(String, Option<Presence>),
}

static WORKER: OnceLock<Mutex<Sender<Msg>>> = OnceLock::new();

fn worker() -> Sender<Msg> {
    WORKER
        .get_or_init(|| {
            let (tx, rx) = channel::<Msg>();
            std::thread::Builder::new()
                .name("discord".into())
                .spawn(move || {
                    let mut client: Option<(String, DiscordIpcClient)> = None;
                    let mut want: Option<(String, Option<Presence>)> = None;
                    let mut shown: Option<(String, Option<Presence>)> = None;
                    loop {
                        match rx.recv_timeout(Duration::from_secs(15)) {
                            Ok(Msg::Set(id, p)) => want = Some((id, p)),
                            Err(RecvTimeoutError::Timeout) => {}
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                        let Some((id, presence)) = want.clone() else { continue };
                        if shown.as_ref() == Some(&(id.clone(), presence.clone())) {
                            continue;
                        }
                        // a different app id means a new connection
                        if client.as_ref().map(|(cid, _)| cid != &id).unwrap_or(false) {
                            if let Some((_, mut c)) = client.take() {
                                let _ = c.close();
                            }
                        }
                        if client.is_none() {
                            if presence.is_none() {
                                shown = Some((id, None));
                                continue;
                            }
                            let mut c = DiscordIpcClient::new(&id);
                            if c.connect().is_err() {
                                continue; // Discord isn't running: try again in 15 s
                            }
                            client = Some((id.clone(), c));
                        }
                        let Some((_, c)) = client.as_mut() else { continue };
                        let res = match &presence {
                            Some(p) => {
                                let mut a = Activity::new()
                                    .details(p.details.as_str())
                                    .state(p.state.as_str())
                                    .assets(Assets::new().large_image("logo").large_text(p.large_text.as_str()));
                                if let Some(start) = p.start {
                                    a = a.timestamps(Timestamps::new().start(start * 1000));
                                }
                                c.set_activity(a)
                            }
                            None => c.clear_activity(),
                        };
                        if res.is_ok() {
                            shown = Some((id, presence));
                        } else {
                            // Discord was closed: drop the connection, retry later
                            if let Some((_, mut c)) = client.take() {
                                let _ = c.close();
                            }
                            shown = None;
                        }
                    }
                })
                .expect("discord thread");
            Mutex::new(tx)
        })
        .lock()
        .map(|t| t.clone())
        .expect("discord worker")
}

fn app_id(state: &AppState) -> String {
    state
        .settings()
        .pointer("/discord/appId")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(default_app_id())
        .to_string()
}

/// Shows (or with `None` clears) the status.
#[tauri::command(async)]
pub fn discord_presence(state: State<'_, AppState>, presence: Option<Presence>) {
    let _ = worker().send(Msg::Set(app_id(&state), presence));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a Discord (or /tmp/fake_discord.py) listening on the IPC socket.
    #[test]
    #[ignore]
    fn talks_to_discord() {
        let p = Presence { details: "Играет: Выживание".into(), state: "Fabric 1.21.11 · В мире".into(), start: Some(1_794_800_000), large_text: "AniLauncher".into() };
        worker().send(Msg::Set(default_app_id().into(), Some(p))).unwrap();
        std::thread::sleep(Duration::from_millis(800));
        worker().send(Msg::Set(default_app_id().into(), None)).unwrap();
        std::thread::sleep(Duration::from_millis(800));
    }
}
