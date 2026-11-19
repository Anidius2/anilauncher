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

fn is_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
}

/// Screenshots of deleted builds: `<root>/archive/<id>-<time>/screenshots`, with the build's name next to them.
fn archive_dir(state: &AppState) -> PathBuf {
    state.root.join("archive")
}

/// Moves a build's screenshots out before the build is deleted. Never fails the delete.
pub fn archive_screenshots(state: &AppState, id: &str, name: &str) {
    let src = instances::game_dir(state, id).join("screenshots");
    let files: Vec<PathBuf> = std::fs::read_dir(&src)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.file_name().map(|n| is_image(&n.to_string_lossy())).unwrap_or(false))
        .collect();
    if files.is_empty() {
        return;
    }
    let dest = archive_dir(state).join(format!("{id}-{}", now())).join("screenshots");
    if std::fs::create_dir_all(&dest).is_err() {
        return;
    }
    let _ = std::fs::write(dest.parent().unwrap().join("name.txt"), name);
    for f in files {
        let to = dest.join(f.file_name().unwrap());
        if std::fs::rename(&f, &to).is_err() {
            let _ = std::fs::copy(&f, &to);
        }
    }
}

fn push_shots(out: &mut Vec<Shot>, dir: &Path, id: &str, name: &str) {
    for f in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = f.path();
        let file = f.file_name().to_string_lossy().to_string();
        if !is_image(&file) {
            continue;
        }
        out.push(Shot {
            modified: modified(&path),
            size: f.metadata().map(|m| m.len()).unwrap_or(0),
            path: path.to_string_lossy().into(),
            name: file,
            instance_id: id.to_string(),
            instance_name: name.to_string(),
        });
    }
}

#[tauri::command(async)]
pub fn list_screenshots(state: State<'_, AppState>) -> Vec<Shot> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(archive_dir(&state)).into_iter().flatten().flatten() {
        let key = e.file_name().to_string_lossy().to_string();
        let name = std::fs::read_to_string(e.path().join("name.txt")).unwrap_or_else(|_| key.clone());
        push_shots(&mut out, &e.path().join("screenshots"), &format!("archive:{key}"), &name);
    }
    for e in std::fs::read_dir(state.instances()).into_iter().flatten().flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let Ok(inst) = instances::load(&state, &id) else { continue };
        push_shots(&mut out, &instances::game_dir(&state, &id).join("screenshots"), &id, &inst.name);
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

#[tauri::command(async)]
pub fn copy_image<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, path: String) -> Res<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let p = inside_root(&state, &path)?;
    let img = tauri::image::Image::from_path(&p).ctx("Не удалось открыть картинку")?;
    app.clipboard().write_image(&img).ctx("Не удалось скопировать")
}

/// While a game runs, every new screenshot (F2) goes straight to the clipboard, and the game says so.
/// The game writes screenshots on a background thread, so a file is taken only once it stops growing.
pub fn watch_screenshots<R: Runtime>(app: AppHandle<R>, instance: String, dir: PathBuf) {
    std::thread::spawn(move || {
        use tauri::{Emitter, Manager};
        use tauri_plugin_clipboard_manager::ClipboardExt;
        let pngs = |d: &Path| -> Vec<(String, u64)> {
            std::fs::read_dir(d)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let size = e.metadata().ok()?.len();
                    name.to_lowercase().ends_with(".png").then_some((name, size))
                })
                .collect()
        };
        let mut seen: std::collections::HashSet<String> = pngs(&dir).into_iter().map(|(n, _)| n).collect();
        let mut growing: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let st = app.state::<AppState>();
            if !st.running.lock().map(|r| r.contains_key(&instance)).unwrap_or(false) {
                break;
            }
            for (name, size) in pngs(&dir) {
                if seen.contains(&name) || size == 0 {
                    continue;
                }
                if growing.get(&name) != Some(&size) {
                    growing.insert(name, size); // still being written: look again next time
                    continue;
                }
                growing.remove(&name);
                seen.insert(name.clone());
                let Ok(img) = tauri::image::Image::from_path(dir.join(&name)) else { continue };
                if app.clipboard().write_image(&img).is_err() {
                    continue;
                }
                let en = st.settings().get("lang").and_then(|v| v.as_str()) == Some("en");
                let text = if en { "Screenshot copied to the clipboard" } else { "Скриншот скопирован в буфер обмена" };
                let hub = st.link.clone();
                let inst = instance.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = hub.send(&inst, serde_json::json!({"cmd": "toast", "title": "AniLauncher", "text": text})).await;
                });
                let _ = app.emit("screenshot-copied", serde_json::json!({"instanceId": instance, "file": name}));
            }
        }
    });
}

/// Saves a copy of a launcher file to a place the user picked.
#[tauri::command(async)]
pub fn save_copy(state: State<'_, AppState>, path: String, dest: String) -> Res<()> {
    let p = inside_root(&state, &path)?;
    std::fs::copy(&p, &dest).ctx("Не удалось сохранить копию")?;
    Ok(())
}

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
pub fn delete_backup(state: State<'_, AppState>, instance_id: String, file: String) -> Res<()> {
    valid_world(&file)?;
    std::fs::remove_file(backups_dir(&state, &instance_id).join(file)).ctx("Не удалось удалить бэкап")
}

#[cfg(test)]
mod archive_tests {
    #[test]
    fn screenshots_survive_a_deleted_build() {
        use tauri::Manager;
        let root = std::env::temp_dir().join(format!("anilauncher-archive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(crate::state::AppState::with_root(root.clone()));
        let st = app.state::<crate::state::AppState>();
        let inst = crate::instances::Instance { id: "old".into(), name: "Старая сборка".into(), mc_version: "1.21.11".into(), ..Default::default() };
        crate::instances::store(&st, &inst).unwrap();
        let shots = crate::instances::game_dir(&st, "old").join("screenshots");
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(shots.join("2026-11-18_17.00.00.png"), b"png").unwrap();
        std::fs::write(shots.join("notes.txt"), b"x").unwrap();

        crate::instances::delete_instance(st.clone(), "old".into()).unwrap();
        assert!(!crate::instances::dir(&st, "old").exists());
        let list = super::list_screenshots(st.clone());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].instance_name, "Старая сборка");
        assert!(list[0].instance_id.starts_with("archive:old-"));
        // and it can still be deleted from the gallery
        super::delete_screenshot(st.clone(), list[0].path.clone()).unwrap();
        assert!(super::list_screenshots(st).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
