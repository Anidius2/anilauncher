//! Screenshot gallery across all builds, and world backups.

use crate::instances;
use crate::state::AppState;
use crate::util::{now, Ctx, Res};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Runtime, State};

/// Only files inside the launcher folder may be touched by these commands.
fn inside_root(state: &AppState, path: &str) -> Res<PathBuf> {
    let p = std::fs::canonicalize(path).ctx("Файл не найден")?;
    let root = std::fs::canonicalize(&state.root).ctx("Папка лаунчера не найдена")?;
    if !p.starts_with(&root) {
        return Err("Этот файл не из лаунчера".into());
    }
    Ok(p)
}

fn modified(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    path: String,
    name: String,
    instance_id: String,
    instance_name: String,
    modified: u64,
    size: u64,
}

#[tauri::command]
pub fn list_screenshots(state: State<'_, AppState>) -> Vec<Shot> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(state.instances()).into_iter().flatten().flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let Ok(inst) = instances::load(&state, &id) else { continue };
        let dir = instances::game_dir(&state, &id).join("screenshots");
        for f in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = f.path();
            let name = f.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if !(lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")) {
                continue;
            }
            out.push(Shot {
                modified: modified(&path),
                size: f.metadata().map(|m| m.len()).unwrap_or(0),
                path: path.to_string_lossy().into(),
                name,
                instance_id: id.clone(),
                instance_name: inst.name.clone(),
            });
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

#[tauri::command]
pub fn copy_image<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, path: String) -> Res<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let p = inside_root(&state, &path)?;
    let img = tauri::image::Image::from_path(&p).ctx("Не удалось открыть картинку")?;
    app.clipboard().write_image(&img).ctx("Не удалось скопировать")
}

/// Saves a copy of a launcher file to a place the user picked.
#[tauri::command]
pub fn save_copy(state: State<'_, AppState>, path: String, dest: String) -> Res<()> {
    let p = inside_root(&state, &path)?;
    std::fs::copy(&p, &dest).ctx("Не удалось сохранить копию")?;
    Ok(())
}

#[tauri::command]
pub fn delete_screenshot(state: State<'_, AppState>, path: String) -> Res<()> {
    let p = inside_root(&state, &path)?;
    if p.parent().and_then(|d| d.file_name()).map(|n| n != "screenshots").unwrap_or(true) {
        return Err("Это не скриншот".into());
    }
    std::fs::remove_file(&p).ctx("Не удалось удалить")
}

// ---------- worlds & backups ----------

fn dir_size(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                dir_size(&p)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    name: String,
    size: u64,
    modified: u64,
    icon: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    file: String,
    world: String,
    created: u64,
    size: u64,
}

fn valid_world(name: &str) -> Res<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Неверное имя мира".into());
    }
    Ok(())
}

#[tauri::command]
pub fn list_worlds(state: State<'_, AppState>, instance_id: String) -> Vec<World> {
    let saves = instances::game_dir(&state, &instance_id).join("saves");
    let mut out: Vec<World> = std::fs::read_dir(&saves)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join("level.dat").exists())
        .map(|e| {
            let p = e.path();
            let icon = p.join("icon.png");
            World {
                name: e.file_name().to_string_lossy().into(),
                size: dir_size(&p),
                modified: modified(&p.join("level.dat")),
                icon: icon.exists().then(|| icon.to_string_lossy().into()),
            }
        })
        .collect();
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

fn backups_dir(state: &AppState, id: &str) -> PathBuf {
    instances::game_dir(state, id).join("backups")
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>, instance_id: String) -> Vec<Backup> {
    let mut out: Vec<Backup> = std::fs::read_dir(backups_dir(&state, &instance_id))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().to_string();
            let stem = file.strip_suffix(".zip")?;
            let (world, ts) = stem.rsplit_once("__")?;
            Some(Backup {
                world: world.to_string(),
                created: ts.parse().unwrap_or(0),
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
                file,
            })
        })
        .collect();
    out.sort_by(|a, b| b.created.cmp(&a.created));
    out
}

fn zip_dir(zip: &mut zip::ZipWriter<std::fs::File>, dir: &Path, prefix: &str) -> Res<()> {
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        let name = format!("{prefix}{}", e.file_name().to_string_lossy());
        if p.is_dir() {
            zip_dir(zip, &p, &format!("{name}/"))?;
        } else {
            if name.ends_with("session.lock") {
                continue;
            }
            zip.start_file(name, opts).ctx("Не удалось записать бэкап")?;
            let bytes = std::fs::read(&p).ctx("Не удалось прочитать мир")?;
            zip.write_all(&bytes).ctx("Не удалось записать бэкап")?;
        }
    }
    Ok(())
}

pub fn backup_world_inner(state: &AppState, instance_id: &str, world: &str) -> Res<String> {
    valid_world(world)?;
    let src = instances::game_dir(state, instance_id).join("saves").join(world);
    if !src.join("level.dat").exists() {
        return Err("Мир не найден".into());
    }
    let dir = backups_dir(state, instance_id);
    std::fs::create_dir_all(&dir).ctx("Не удалось создать папку бэкапов")?;
    // Unique name even when two backups happen within the same second
    let mut stamp = now();
    while dir.join(format!("{world}__{stamp}.zip")).exists() {
        stamp += 1;
    }
    let file = format!("{world}__{stamp}.zip");
    let tmp = dir.join(format!("{file}.part"));
    let out = std::fs::File::create(&tmp).ctx("Не удалось создать бэкап")?;
    let mut zip = zip::ZipWriter::new(out);
    zip_dir(&mut zip, &src, &format!("{world}/"))?;
    zip.finish().ctx("Не удалось записать бэкап")?;
    std::fs::rename(&tmp, dir.join(&file)).ctx("Не удалось сохранить бэкап")?;
    Ok(file)
}

#[tauri::command]
pub async fn backup_world(state: State<'_, AppState>, instance_id: String, world: String) -> Res<String> {
    backup_world_inner(&state, &instance_id, &world)
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, instance_id: String, file: String) -> Res<()> {
    if state.running.lock().map(|r| r.contains_key(&instance_id)).unwrap_or(false) {
        return Err("Сначала закрой игру".into());
    }
    valid_world(&file)?;
    let zip_path = backups_dir(&state, &instance_id).join(&file);
    let world = file
        .strip_suffix(".zip")
        .and_then(|s| s.rsplit_once("__"))
        .map(|(w, _)| w.to_string())
        .ok_or("Неверный файл бэкапа")?;
    let saves = instances::game_dir(&state, &instance_id).join("saves");
    let current = saves.join(&world);
    if current.exists() {
        // Keep the current world too, just in case
        let _ = backup_world_inner(&state, &instance_id, &world);
        std::fs::remove_dir_all(&current).ctx("Не удалось заменить мир")?;
    }
    let f = std::fs::File::open(&zip_path).ctx("Бэкап не найден")?;
    let mut zip = zip::ZipArchive::new(f).ctx("Повреждённый бэкап")?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).ctx("Повреждённый бэкап")?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = saves.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).ctx("Не удалось восстановить")?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).ctx("Не удалось восстановить")?;
        }
        let mut w = std::fs::File::create(&out).ctx("Не удалось восстановить")?;
        std::io::copy(&mut entry, &mut w).ctx("Не удалось восстановить")?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_backup(state: State<'_, AppState>, instance_id: String, file: String) -> Res<()> {
    valid_world(&file)?;
    std::fs::remove_file(backups_dir(&state, &instance_id).join(file)).ctx("Не удалось удалить бэкап")
}
