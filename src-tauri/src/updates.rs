//! Updates of the launcher itself, from the GitHub releases of its own repository. Each release
//! carries a signature; the launcher only installs what was signed with the project's key.

use crate::state::AppState;
use crate::util::{Ctx, Res};
use serde::Serialize;
use serde_json::json;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Public half of the signing key (the private half lives only in the repository's secrets).
pub const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDUyNkQzQTlDRTZDMkRCODgKUldTSTI4TG1uRHB0VW4yTUhEYUNoRHFZS2EwMlZNSHM3akpBQ1dES2xYcWtlU1VpZGlmVFpHNVkK";

/// Where releases are published; the GitHub build fills it in for its own repository.
fn endpoint() -> Option<&'static str> {
    option_env!("ANILAUNCHER_UPDATE_URL").filter(|u| u.starts_with("https://"))
}

#[derive(Default)]
pub struct Pending(pub Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    current: String,
    notes: Option<String>,
    date: Option<String>,
}

/// None: up to date (or updates are not set up in this build).
#[tauri::command]
pub async fn check_update<R: Runtime>(app: AppHandle<R>) -> Res<Option<UpdateInfo>> {
    let Some(url) = endpoint() else { return Ok(None) };
    let updater = app
        .updater_builder()
        .endpoints(vec![url.parse().ctx("Неверный адрес обновлений")?])
        .ctx("Неверный адрес обновлений")?
        .build()
        .ctx("Обновления недоступны")?;
    let found = updater.check().await.map_err(|e| format!("Не удалось проверить обновления: {e}"))?;
    let info = found.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current: u.current_version.clone(),
        notes: u.body.clone(),
        date: u.date.map(|d| d.to_string()),
    });
    if let Some(p) = app.try_state::<Pending>() {
        if let Ok(mut slot) = p.0.lock() {
            *slot = found;
        }
    }
    Ok(info)
}

#[tauri::command]
pub fn updates_configured() -> bool {
    endpoint().is_some()
}

/// Downloads the found update (with progress events), installs it and restarts.
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Res<()> {
    if state.running.lock().map(|r| !r.is_empty()).unwrap_or(false) {
        return Err("Сначала закрой игру — во время обновления лаунчер перезапустится".into());
    }
    let update = app
        .try_state::<Pending>()
        .and_then(|p| p.0.lock().ok().and_then(|mut s| s.take()))
        .ok_or("Сначала проверь обновления")?;
    let app_p = app.clone();
    let mut done: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let _ = app_p.emit("update-progress", json!({"done": done, "total": total}));
            },
            || {},
        )
        .await
        .map_err(|e| format!("Не удалось установить обновление: {e}"))?;
    app.restart();
}
