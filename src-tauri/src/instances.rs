//! Builds (instances): each has its own folder, game files, art and settings.

use crate::state::AppState;
use crate::util::{now, read_json, write_json, Ctx, Res};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Art {
    /// "url" (official Mojang art) or "file" (own picture)
    pub kind: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    /// "vanilla", "fabric" or "quilt"
    pub loader: String,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub art: Option<Art>,
    #[serde(default)]
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub jvm_args: Option<String>,
    pub created: u64,
    #[serde(default)]
    pub last_played: Option<u64>,
    #[serde(default)]
    pub play_seconds: u64,
    /// Game window size (None = Minecraft default)
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fullscreen: bool,
    /// Server to join right after the game starts (host or host:port)
    #[serde(default)]
    pub server: Option<String>,
    /// Own Java instead of the one the launcher downloads
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    /// Servers joined recently (newest first), read from the game log
    #[serde(default)]
    pub recent_servers: Vec<RecentServer>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecentServer {
    pub address: String,
    pub time: u64,
}

pub fn dir(state: &AppState, id: &str) -> PathBuf {
    state.instances().join(id)
}

pub fn game_dir(state: &AppState, id: &str) -> PathBuf {
    dir(state, id).join("minecraft")
}

/// Same as [`game_dir`] when only the launcher folder is at hand.
pub fn game_dir_at(root: &std::path::Path, id: &str) -> PathBuf {
    root.join("instances").join(id).join("minecraft")
}

pub fn load(state: &AppState, id: &str) -> Res<Instance> {
    read_json(&dir(state, id).join("instance.json")).ok_or_else(|| "Сборка не найдена".to_string())
}

pub fn store(state: &AppState, inst: &Instance) -> Res<()> {
    write_json(&dir(state, &inst.id).join("instance.json"), inst)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[tauri::command(async)]
pub fn list_instances(state: State<'_, AppState>) -> Vec<Instance> {
    let mut list: Vec<Instance> = std::fs::read_dir(state.instances())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| read_json(&e.path().join("instance.json")))
        .collect();
    list.sort_by(|a, b| {
        b.pinned.cmp(&a.pinned).then(
            b.last_played
                .unwrap_or(b.created)
                .cmp(&a.last_played.unwrap_or(a.created)),
        )
    });
    list
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewInstance {
    name: String,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
    art: Option<Art>,
}

#[tauri::command(async)]
pub fn create_instance(state: State<'_, AppState>, data: NewInstance) -> Res<Instance> {
    let name = data.name.trim();
    if name.is_empty() {
        return Err("Дай сборке название".into());
    }
    if !["vanilla", "fabric", "quilt", "forge", "neoforge"].contains(&data.loader.as_str()) {
        return Err("Этот загрузчик пока не поддерживается".into());
    }
    let inst = Instance {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: name.chars().take(48).collect(),
        mc_version: data.mc_version,
        loader: data.loader,
        loader_version: data.loader_version.filter(|v| !v.is_empty()),
        art: data.art,
        memory_mb: None,
        jvm_args: None,
        created: now(),
        ..Default::default()
    };
    std::fs::create_dir_all(game_dir(&state, &inst.id)).ctx("Не удалось создать папку сборки")?;
    store(&state, &inst)?;
    Ok(inst)
}

#[tauri::command(async)]
pub fn update_instance(state: State<'_, AppState>, instance: Instance) -> Res<Instance> {
    if !valid_id(&instance.id) {
        return Err("Неверная сборка".into());
    }
    let mut current = load(&state, &instance.id)?;
    let name = instance.name.trim();
    if !name.is_empty() {
        current.name = name.chars().take(48).collect();
    }
    current.art = instance.art;
    current.memory_mb = instance.memory_mb;
    current.jvm_args = instance.jvm_args;
    if instance.mc_version != current.mc_version || instance.loader != current.loader {
        crate::snapshots::auto(&state, &current.id, "version", &format!("{} {}", current.loader, current.mc_version));
        current.mc_version = instance.mc_version;
        current.loader = instance.loader;
    }
    current.loader_version = instance.loader_version.filter(|v| !v.is_empty());
    current.width = instance.width.filter(|w| *w >= 320);
    current.height = instance.height.filter(|h| *h >= 240);
    current.fullscreen = instance.fullscreen;
    current.server = instance.server.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    current.java_path = instance.java_path.filter(|p| !p.trim().is_empty());
    current.pinned = instance.pinned;
    store(&state, &current)?;
    Ok(current)
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> Res<()> {
    std::fs::create_dir_all(to).ctx("Не удалось создать папку")?;
    for e in std::fs::read_dir(from).ctx("Не удалось прочитать папку")?.flatten() {
        let src = e.path();
        let dest = to.join(e.file_name());
        if src.is_dir() {
            copy_dir(&src, &dest)?;
        } else {
            std::fs::copy(&src, &dest).ctx("Не удалось скопировать файл")?;
        }
    }
    Ok(())
}

/// Full copy of a build (game folder, mods, worlds, settings) under a new name.
#[tauri::command]
pub async fn duplicate_instance(state: State<'_, AppState>, id: String) -> Res<Instance> {
    if !valid_id(&id) {
        return Err("Неверная сборка".into());
    }
    let mut inst = load(&state, &id)?;
    let new_id = uuid::Uuid::new_v4().simple().to_string();
    copy_dir(&dir(&state, &id), &dir(&state, &new_id))?;
    inst.id = new_id;
    inst.name = format!("{} (копия)", inst.name).chars().take(48).collect();
    inst.created = now();
    inst.last_played = None;
    inst.play_seconds = 0;
    inst.pinned = false;
    store(&state, &inst)?;
    Ok(inst)
}

/// The last game output of a build (for the console).
#[tauri::command(async)]
pub fn read_launch_log(state: State<'_, AppState>, id: String) -> Vec<String> {
    if !valid_id(&id) {
        return vec![];
    }
    let text = std::fs::read_to_string(dir(&state, &id).join("logs").join("latest-launch.log")).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(4000)..].iter().map(|s| s.to_string()).collect()
}

#[tauri::command(async)]
pub fn delete_instance(state: State<'_, AppState>, id: String) -> Res<()> {
    if !valid_id(&id) {
        return Err("Неверная сборка".into());
    }
    if state.running.lock().map(|r| r.contains_key(&id)).unwrap_or(false) {
        return Err("Сначала закрой игру".into());
    }
    // screenshots outlive the build: they move to the launcher's archive and stay in the gallery
    if let Ok(inst) = load(&state, &id) {
        crate::media::archive_screenshots(&state, &id, &inst.name);
    }
    std::fs::remove_dir_all(dir(&state, &id)).ctx("Не удалось удалить сборку")
}

/// Opens the build's game folder (or a subfolder like mods, saves, screenshots).
#[tauri::command(async)]
pub fn open_instance_folder(app: AppHandle, state: State<'_, AppState>, id: String, sub: Option<String>) -> Res<()> {
    if !valid_id(&id) {
        return Err("Неверная сборка".into());
    }
    let mut path = game_dir(&state, &id);
    if let Some(sub) = sub {
        if ["mods", "saves", "resourcepacks", "shaderpacks", "screenshots", "logs", "backups", "config"].contains(&sub.as_str()) {
            path = path.join(sub);
        }
    }
    std::fs::create_dir_all(&path).ctx("Не удалось создать папку")?;
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .ctx("Не удалось открыть папку")
}
