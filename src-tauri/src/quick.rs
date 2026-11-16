//! "Continue where you left off": last worlds and servers across builds, and key binding profiles.

use crate::instances::{self, RecentServer};
use crate::state::AppState;
use crate::util::{now, read_json, write_json, Ctx, Res};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tauri::State;

/// Finds the last "Connecting to host, port" line of a game log.
pub fn last_server(log: &str) -> Option<String> {
    let re = regex::Regex::new(r"Connecting to ([^\s,]+),\s*(\d+)").ok()?;
    let cap = re.captures_iter(log).last()?;
    let host = cap.get(1)?.as_str().trim_end_matches('.');
    let port = cap.get(2)?.as_str();
    Some(if port == "25565" { host.to_string() } else { format!("{host}:{port}") })
}

pub fn remember_server(state: &AppState, id: &str, log_path: &Path) {
    let Ok(text) = std::fs::read_to_string(log_path) else { return };
    let Some(addr) = last_server(&text) else { return };
    if let Ok(mut inst) = instances::load(state, id) {
        inst.recent_servers.retain(|s| s.address != addr);
        inst.recent_servers.insert(0, RecentServer { address: addr, time: now() });
        inst.recent_servers.truncate(5);
        let _ = instances::store(state, &inst);
    }
}

/// Minecraft 1.20+ can open a world straight from the launcher.
pub fn supports_quick_world(mc: &str) -> bool {
    let parts: Vec<u32> = mc.split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect();
    match parts.as_slice() {
        [1, minor, ..] => *minor >= 20,
        [major, ..] => *major >= 20,
        _ => false,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickItem {
    /// "world" or "server"
    kind: String,
    instance_id: String,
    instance_name: String,
    target: String,
    time: u64,
    icon: Option<String>,
}

#[tauri::command(async)]
pub fn quick_start(state: State<'_, AppState>) -> Vec<QuickItem> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(state.instances()).into_iter().flatten().flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let Ok(inst) = instances::load(&state, &id) else { continue };
        if supports_quick_world(&inst.mc_version) {
            let saves = instances::game_dir(&state, &id).join("saves");
            let newest = std::fs::read_dir(&saves)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|w| {
                    let dat = w.path().join("level.dat");
                    let t = dat.metadata().ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();
                    Some((t, w.file_name().to_string_lossy().to_string(), w.path().join("icon.png")))
                })
                .max_by_key(|x| x.0);
            if let Some((t, name, icon)) = newest {
                out.push(QuickItem {
                    kind: "world".into(),
                    instance_id: id.clone(),
                    instance_name: inst.name.clone(),
                    target: name,
                    time: t,
                    icon: icon.exists().then(|| icon.to_string_lossy().to_string()),
                });
            }
        }
        if let Some(s) = inst.recent_servers.first() {
            out.push(QuickItem {
                kind: "server".into(),
                instance_id: id.clone(),
                instance_name: inst.name.clone(),
                target: s.address.clone(),
                time: s.time,
                icon: None,
            });
        }
    }
    out.sort_by(|a, b| b.time.cmp(&a.time));
    out.truncate(6);
    out
}

// ---------- key binding profiles ----------

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ControlProfile {
    name: String,
    keys: BTreeMap<String, String>,
    created: u64,
    from: String,
}

fn profiles_path(state: &AppState) -> std::path::PathBuf {
    state.root.join("controls.json")
}

fn read_keys(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.starts_with("key_").then(|| (k.to_string(), v.to_string()))
        })
        .collect()
}

#[tauri::command(async)]
pub fn list_controls(state: State<'_, AppState>) -> Vec<ControlProfile> {
    read_json(&profiles_path(&state)).unwrap_or_default()
}

/// Saves the key bindings of a build as a named profile.
#[tauri::command(async)]
pub fn save_controls(state: State<'_, AppState>, instance_id: String, name: String) -> Res<Vec<ControlProfile>> {
    let inst = instances::load(&state, &instance_id)?;
    let keys = read_keys(&instances::game_dir(&state, &instance_id).join("options.txt"));
    if keys.is_empty() {
        return Err("В этой сборке ещё нет настроек управления — запусти её хотя бы раз".into());
    }
    let mut list: Vec<ControlProfile> = read_json(&profiles_path(&state)).unwrap_or_default();
    let name = name.trim().chars().take(40).collect::<String>();
    let name = if name.is_empty() { inst.name.clone() } else { name };
    list.retain(|p| p.name != name);
    list.insert(0, ControlProfile { name, keys, created: now(), from: inst.name });
    write_json(&profiles_path(&state), &list)?;
    Ok(list)
}

/// Writes a profile's key bindings into a build's options.txt (other settings stay untouched).
#[tauri::command(async)]
pub fn apply_controls(state: State<'_, AppState>, instance_id: String, name: String) -> Res<usize> {
    if state.running.lock().map(|r| r.contains_key(&instance_id)).unwrap_or(false) {
        return Err("Сначала закрой игру — она перезапишет настройки при выходе".into());
    }
    let list: Vec<ControlProfile> = read_json(&profiles_path(&state)).unwrap_or_default();
    let profile = list.iter().find(|p| p.name == name).ok_or("Профиль не найден")?;
    let path = instances::game_dir(&state, &instance_id).join("options.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().filter(|l| !l.starts_with("key_")).map(String::from).collect();
    for (k, v) in &profile.keys {
        lines.push(format!("{k}:{v}"));
    }
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).ctx("Не удалось создать папку")?;
    }
    std::fs::write(&path, lines.join("\n") + "\n").ctx("Не удалось сохранить управление")?;
    Ok(profile.keys.len())
}

#[tauri::command(async)]
pub fn delete_controls(state: State<'_, AppState>, name: String) -> Res<Vec<ControlProfile>> {
    let mut list: Vec<ControlProfile> = read_json(&profiles_path(&state)).unwrap_or_default();
    list.retain(|p| p.name != name);
    write_json(&profiles_path(&state), &list)?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_server_and_versions() {
        let log = "[12:00] Connecting to old.example.net, 25565\n[12:30] [Render thread/INFO]: Connecting to play.example.net, 25570\n";
        assert_eq!(last_server(log).as_deref(), Some("play.example.net:25570"));
        assert_eq!(last_server("Connecting to mc.host.org, 25565").as_deref(), Some("mc.host.org"));
        assert!(supports_quick_world("1.20.1"));
        assert!(supports_quick_world("26.3"));
        assert!(!supports_quick_world("1.19.4"));
        assert!(!supports_quick_world("1.12.2"));
    }
}
