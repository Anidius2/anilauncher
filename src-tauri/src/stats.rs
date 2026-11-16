//! Play sessions (for the weekly chart and the activity calendar) and the newest game versions.

use crate::net::get_json;
use crate::state::AppState;
use crate::util::{read_json, write_json, Res};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub instance_id: String,
    pub start: u64,
    pub end: u64,
}

fn path(state: &AppState) -> std::path::PathBuf {
    state.root.join("sessions.json")
}

pub fn record(state: &AppState, instance_id: &str, start: u64, end: u64) {
    if end <= start + 5 {
        return;
    }
    let mut list: Vec<Session> = read_json(&path(state)).unwrap_or_default();
    list.push(Session { instance_id: instance_id.into(), start, end });
    let n = list.len();
    if n > 20000 {
        list.drain(0..n - 20000);
    }
    let _ = write_json(&path(state), &list);
}

#[tauri::command]
pub fn play_sessions(state: State<'_, AppState>, since: u64) -> Vec<Session> {
    read_json::<Vec<Session>>(&path(&state))
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.end >= since)
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    release: String,
    snapshot: String,
    release_time: String,
    snapshot_time: String,
}

#[tauri::command]
pub async fn minecraft_latest(state: State<'_, AppState>) -> Res<Latest> {
    let m: Value = get_json(&state.http, "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json").await?;
    let release = m.pointer("/latest/release").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let snapshot = m.pointer("/latest/snapshot").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let time_of = |id: &str| {
        m.get("versions")
            .and_then(|v| v.as_array())
            .and_then(|a| a.iter().find(|v| v.get("id").and_then(|i| i.as_str()) == Some(id)))
            .and_then(|v| v.get("releaseTime").and_then(|t| t.as_str()))
            .unwrap_or("")
            .to_string()
    };
    Ok(Latest { release_time: time_of(&release), snapshot_time: time_of(&snapshot), release, snapshot })
}
