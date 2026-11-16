//! Restore points for a build: mods, configs and launcher settings, so a bad update
//! can be undone with one click. Worlds are not included (they have their own backups).

use crate::instances;
use crate::state::AppState;
use crate::util::{now, read_json, write_json, Ctx, Res};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::State;

/// What a restore point contains, relative to the game folder.
const DIRS: [&str; 7] = ["mods", "config", "defaultconfigs", "kubejs", "resourcepacks", "shaderpacks", "global_packs"];
const FILES: [&str; 3] = ["options.txt", "optionsof.txt", "optionsshaders.txt"];
const KEEP_AUTO: usize = 15;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub file: String,
    pub created: u64,
    /// manual | update | optimize | version | restore
    pub reason: String,
    pub label: String,
    pub size: u64,
    pub mods: usize,
}

fn dir(state: &AppState, id: &str) -> PathBuf {
    instances::dir(state, id).join("snapshots")
}

fn list_path(state: &AppState, id: &str) -> PathBuf {
    dir(state, id).join("snapshots.json")
}

fn load(state: &AppState, id: &str) -> Vec<Snapshot> {
    read_json(&list_path(state, id)).unwrap_or_default()
}

fn add_dir(zip: &mut zip::ZipWriter<std::fs::File>, path: &Path, prefix: &str) -> Res<()> {
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(true);
    for e in std::fs::read_dir(path).into_iter().flatten().flatten() {
        let p = e.path();
        let name = format!("{prefix}{}", e.file_name().to_string_lossy());
        if p.is_dir() {
            add_dir(zip, &p, &format!("{name}/"))?;
        } else {
            zip.start_file(name, opts).ctx("Не удалось записать точку восстановления")?;
            zip.write_all(&std::fs::read(&p).ctx("Не удалось прочитать файл")?)
                .ctx("Не удалось записать точку восстановления")?;
        }
    }
    Ok(())
}

pub fn create(state: &AppState, id: &str, reason: &str, label: &str) -> Res<Snapshot> {
    let game = instances::game_dir(state, id);
    let d = dir(state, id);
    std::fs::create_dir_all(&d).ctx("Не удалось создать папку")?;
    let mut stamp = now();
    while d.join(format!("{stamp}.zip")).exists() {
        stamp += 1;
    }
    let file = format!("{stamp}.zip");
    let tmp = d.join(format!("{file}.part"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp).ctx("Не удалось создать точку восстановления")?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for name in DIRS {
        let p = game.join(name);
        if p.is_dir() {
            add_dir(&mut zip, &p, &format!("game/{name}/"))?;
        }
    }
    for name in FILES {
        let p = game.join(name);
        if p.is_file() {
            zip.start_file(format!("game/{name}"), opts).ctx("Ошибка записи")?;
            zip.write_all(&std::fs::read(&p).ctx("Ошибка чтения")?).ctx("Ошибка записи")?;
        }
    }
    for name in ["instance.json", "content.json"] {
        let p = instances::dir(state, id).join(name);
        if p.is_file() {
            zip.start_file(format!("launcher/{name}"), opts).ctx("Ошибка записи")?;
            zip.write_all(&std::fs::read(&p).ctx("Ошибка чтения")?).ctx("Ошибка записи")?;
        }
    }
    zip.finish().ctx("Не удалось записать точку восстановления")?;
    std::fs::rename(&tmp, d.join(&file)).ctx("Не удалось сохранить точку восстановления")?;
    let snap = Snapshot {
        size: std::fs::metadata(d.join(&file)).map(|m| m.len()).unwrap_or(0),
        mods: std::fs::read_dir(game.join("mods")).map(|r| r.count()).unwrap_or(0),
        file,
        created: stamp,
        reason: reason.into(),
        label: label.chars().take(60).collect(),
    };
    let mut list = load(state, id);
    list.insert(0, snap.clone());
    // Keep the newest automatic points; manual ones stay until deleted
    let mut auto = 0;
    let mut keep = Vec::new();
    for s in list {
        if s.reason != "manual" {
            auto += 1;
            if auto > KEEP_AUTO {
                let _ = std::fs::remove_file(d.join(&s.file));
                continue;
            }
        }
        keep.push(s);
    }
    write_json(&list_path(state, id), &keep)?;
    Ok(snap)
}

/// Automatic point before risky changes; failures never block the change itself.
pub fn auto(state: &AppState, id: &str, reason: &str, label: &str) {
    let has_mods = std::fs::read_dir(instances::game_dir(state, id).join("mods"))
        .map(|mut r| r.next().is_some())
        .unwrap_or(false);
    if has_mods {
        let _ = create(state, id, reason, label);
    }
}

#[tauri::command(async)]
pub fn list_snapshots(state: State<'_, AppState>, instance_id: String) -> Vec<Snapshot> {
    load(&state, &instance_id)
        .into_iter()
        .filter(|s| dir(&state, &instance_id).join(&s.file).exists())
        .collect()
}

#[tauri::command]
pub async fn create_snapshot(state: State<'_, AppState>, instance_id: String, label: String) -> Res<Snapshot> {
    instances::load(&state, &instance_id)?;
    create(&state, &instance_id, "manual", &label)
}

#[tauri::command]
pub async fn restore_snapshot(state: State<'_, AppState>, instance_id: String, file: String) -> Res<()> {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err("Неверный файл".into());
    }
    if state.running.lock().map(|r| r.contains_key(&instance_id)).unwrap_or(false) {
        return Err("Сначала закрой игру".into());
    }
    let path = dir(&state, &instance_id).join(&file);
    if !path.exists() {
        return Err("Точка восстановления не найдена".into());
    }
    // So the restore itself can be undone
    let _ = create(&state, &instance_id, "restore", "");
    let current = instances::load(&state, &instance_id)?;
    let game = instances::game_dir(&state, &instance_id);
    for name in DIRS {
        let _ = std::fs::remove_dir_all(game.join(name));
    }
    for name in FILES {
        let _ = std::fs::remove_file(game.join(name));
    }
    let f = std::fs::File::open(&path).ctx("Не удалось открыть точку восстановления")?;
    let mut zip = zip::ZipArchive::new(f).ctx("Повреждённая точка восстановления")?;
    let inst_dir = instances::dir(&state, &instance_id);
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).ctx("Повреждённая точка восстановления")?;
        if e.is_dir() {
            continue;
        }
        let Some(rel) = e.enclosed_name() else { continue };
        let rel = rel.to_path_buf();
        let out = if let Ok(r) = rel.strip_prefix("game") {
            game.join(r)
        } else if let Ok(r) = rel.strip_prefix("launcher") {
            let name = r.to_string_lossy().to_string();
            if name != "instance.json" && name != "content.json" {
                continue;
            }
            inst_dir.join(name)
        } else {
            continue;
        };
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).ctx("Не удалось восстановить")?;
        }
        let mut w = std::fs::File::create(&out).ctx("Не удалось восстановить")?;
        std::io::copy(&mut e, &mut w).ctx("Не удалось восстановить")?;
    }
    // Only the game/loader versions travel back in time; name, art, playtime stay as they are now
    let restored = instances::load(&state, &instance_id).unwrap_or_else(|_| current.clone());
    let mut inst = current;
    inst.mc_version = restored.mc_version;
    inst.loader = restored.loader;
    inst.loader_version = restored.loader_version;
    instances::store(&state, &inst)?;
    Ok(())
}

#[tauri::command(async)]
pub fn delete_snapshot(state: State<'_, AppState>, instance_id: String, file: String) -> Res<()> {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err("Неверный файл".into());
    }
    let _ = std::fs::remove_file(dir(&state, &instance_id).join(&file));
    let list: Vec<Snapshot> = load(&state, &instance_id).into_iter().filter(|s| s.file != file).collect();
    write_json(&list_path(&state, &instance_id), &list)
}
