//! Backup of the whole launcher into one zip, and restoring it (e.g. after reinstalling Windows).
//! Game files that can be downloaded again (libraries, assets, Java, Minecraft itself) are left out.

use crate::state::AppState;
use crate::util::{now, Ctx, Res};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter, Runtime, State};

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub struct BackupOptions {
    /// worlds and screenshots
    pub worlds: bool,
    /// restore points and world backups made by the launcher
    pub history: bool,
    /// signed-in accounts (contain login tokens)
    pub accounts: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEstimate {
    files: u64,
    bytes: u64,
    builds: usize,
}

/// Inside an instance folder: what to skip, depending on the options.
fn skip_in_instance(rel: &Path, o: BackupOptions) -> bool {
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    let first = parts.first().map(String::as_str).unwrap_or("");
    match first {
        "natives" | "logs" => return true,
        "snapshots" | "backups" => return !o.history,
        "snapshots.json" => return !o.history,
        _ => {}
    }
    if first == "minecraft" {
        let second = parts.get(1).map(String::as_str).unwrap_or("");
        return match second {
            "logs" | "crash-reports" | ".cache" | "debug" | ".fabric" | ".quilt" | "downloads" | ".mixin.out" => true,
            "saves" | "screenshots" | "backups" => !o.worlds,
            _ => false,
        } || rel.extension().map(|e| e == "part" || e == "tmp").unwrap_or(false)
            || rel.file_name().map(|n| n == "session.lock").unwrap_or(false);
    }
    false
}

/// Every file that goes into the backup: (path on disk, name inside the zip).
fn collect(state: &AppState, o: BackupOptions) -> Vec<(PathBuf, String)> {
    let root = &state.root;
    let mut out = Vec::new();
    for f in ["settings.json", "controls.json", "sessions.json"] {
        let p = root.join(f);
        if p.is_file() {
            out.push((p, f.to_string()));
        }
    }
    if o.accounts && root.join("accounts.json").is_file() {
        out.push((root.join("accounts.json"), "accounts.json".into()));
    }
    walk(&root.join("media"), &mut |p| {
        if let Ok(rel) = p.strip_prefix(root) {
            out.push((p.to_path_buf(), slash(rel)));
        }
    });
    for e in std::fs::read_dir(state.instances()).into_iter().flatten().flatten() {
        let dir = e.path();
        if !dir.join("instance.json").is_file() {
            continue;
        }
        walk(&dir, &mut |p| {
            let Ok(rel) = p.strip_prefix(&dir) else { return };
            if skip_in_instance(rel, o) {
                return;
            }
            if let Ok(full) = p.strip_prefix(root) {
                out.push((p.to_path_buf(), slash(full)));
            }
        });
    }
    out
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            walk(&p, f);
        } else if ft.is_file() {
            f(&p);
        }
    }
}

fn slash(p: &Path) -> String {
    p.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect::<Vec<_>>().join("/")
}

#[tauri::command]
pub fn estimate_launcher_backup(state: State<'_, AppState>, options: BackupOptions) -> BackupEstimate {
    let files = collect(&state, options);
    let builds = std::fs::read_dir(state.instances()).into_iter().flatten().flatten().filter(|e| e.path().join("instance.json").is_file()).count();
    BackupEstimate {
        files: files.len() as u64,
        bytes: files.iter().map(|(p, _)| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)).sum(),
        builds,
    }
}

#[tauri::command]
pub async fn create_launcher_backup<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, dest: String, options: BackupOptions) -> Res<u64> {
    let files = collect(&state, options);
    let builds = std::fs::read_dir(state.instances()).into_iter().flatten().flatten().filter(|e| e.path().join("instance.json").is_file()).count();
    let dest = PathBuf::from(dest);
    let tmp = dest.with_extension("zip.part");
    let total = files.len();
    let res = tokio::task::spawn_blocking(move || -> Res<u64> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp).ctx("Не удалось создать файл копии")?);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
        let manifest = json!({
            "app": "AniLauncher",
            "version": env!("CARGO_PKG_VERSION"),
            "created": now(),
            "builds": builds,
            "worlds": options.worlds,
            "history": options.history,
            "accounts": options.accounts,
        });
        zip.start_file("anilauncher-backup.json", opts).ctx("Не удалось записать копию")?;
        zip.write_all(serde_json::to_string_pretty(&manifest).unwrap_or_default().as_bytes()).ctx("Не удалось записать копию")?;
        let mut bytes = 0u64;
        for (i, (path, name)) in files.iter().enumerate() {
            // a file can vanish while we work (the game deleting a temp file): just skip it
            let Ok(mut f) = std::fs::File::open(path) else { continue };
            zip.start_file(name.as_str(), opts).ctx("Не удалось записать копию")?;
            bytes += std::io::copy(&mut f, &mut zip).ctx("Не удалось записать копию")?;
            if i % 25 == 0 || i + 1 == total {
                let _ = app.emit("backup-progress", json!({"done": i + 1, "total": total}));
            }
        }
        zip.finish().ctx("Не удалось записать копию")?;
        std::fs::rename(&tmp, &dest).ctx("Не удалось сохранить копию")?;
        Ok(bytes)
    })
    .await
    .ctx("Копирование прервалось")?;
    res
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    created: u64,
    version: String,
    builds: u64,
    worlds: bool,
    accounts: bool,
}

#[tauri::command]
pub fn inspect_launcher_backup(path: String) -> Res<BackupInfo> {
    let file = std::fs::File::open(&path).ctx("Файл не найден")?;
    let mut zip = zip::ZipArchive::new(file).ctx("Это не архив")?;
    let m: Value = {
        let mut e = zip.by_name("anilauncher-backup.json").map_err(|_| "Это не резервная копия AniLauncher".to_string())?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut e, &mut s).ctx("Копия повреждена")?;
        serde_json::from_str(&s).ctx("Копия повреждена")?
    };
    Ok(BackupInfo {
        created: m["created"].as_u64().unwrap_or(0),
        version: m["version"].as_str().unwrap_or("").into(),
        builds: m["builds"].as_u64().unwrap_or(0),
        worlds: m["worlds"].as_bool().unwrap_or(false),
        accounts: m["accounts"].as_bool().unwrap_or(false),
    })
}

/// Puts everything from the backup back. Existing builds with the same id are overwritten,
/// others stay. Refuses while a game runs.
#[tauri::command]
pub async fn restore_launcher_backup<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, path: String) -> Res<u64> {
    if state.running.lock().map(|r| !r.is_empty()).unwrap_or(false) {
        return Err("Сначала закрой игру".into());
    }
    inspect_launcher_backup(path.clone())?;
    let root = state.root.clone();
    tokio::task::spawn_blocking(move || -> Res<u64> {
        let file = std::fs::File::open(&path).ctx("Файл не найден")?;
        let mut zip = zip::ZipArchive::new(file).ctx("Это не архив")?;
        let total = zip.len();
        let mut restored = 0u64;
        for i in 0..total {
            let mut entry = zip.by_index(i).ctx("Копия повреждена")?;
            let name = entry.name().to_string();
            if name == "anilauncher-backup.json" || entry.is_dir() {
                continue;
            }
            // only plain relative paths inside the launcher folder
            let rel = PathBuf::from(&name);
            if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
                continue;
            }
            let allowed = ["settings.json", "controls.json", "sessions.json", "accounts.json"].contains(&name.as_str())
                || name.starts_with("media/")
                || name.starts_with("instances/");
            if !allowed {
                continue;
            }
            let out = root.join(&rel);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ctx("Не удалось создать папку")?;
            }
            let mut f = std::fs::File::create(&out).ctx("Не удалось записать файл")?;
            std::io::copy(&mut entry, &mut f).ctx("Не удалось записать файл")?;
            restored += 1;
            if i % 25 == 0 || i + 1 == total {
                let _ = app.emit("backup-progress", json!({"done": i + 1, "total": total}));
            }
        }
        Ok(restored)
    })
    .await
    .ctx("Восстановление прервалось")?
}
