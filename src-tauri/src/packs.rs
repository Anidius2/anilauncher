//! Importing builds (.mrpack, Prism/MultiMC zip) and exporting them as .mrpack.

use crate::content::{folder, sha1_file, API};
use crate::instances::{self, Instance};
use crate::net::{download_all, get_json, Download};
use crate::state::AppState;
use crate::util::{now, Ctx, Res};
use crate::versions::default_loader;
use serde_json::{json, Value};
use sha2::{Digest, Sha512};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter, Runtime, State};

/// Joins a relative path from an archive onto `base`, refusing anything that escapes it.
fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    if p.is_absolute() {
        return None;
    }
    let mut out = base.to_path_buf();
    for c in p.components() {
        match c {
            Component::Normal(x) => out.push(x),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

fn new_instance(state: &AppState, name: &str, mc: &str, loader: &str, loader_version: Option<String>) -> Res<Instance> {
    let inst = Instance {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: name.chars().take(48).collect(),
        mc_version: mc.to_string(),
        loader: loader.to_string(),
        loader_version,
        art: None,
        memory_mb: None,
        jvm_args: None,
        created: now(),
        ..Default::default()
    };
    std::fs::create_dir_all(instances::game_dir(state, &inst.id)).ctx("Не удалось создать папку сборки")?;
    instances::store(state, &inst)?;
    Ok(inst)
}

fn emit<R: Runtime>(app: &AppHandle<R>, stage: &str, done: usize, total: usize) {
    let _ = app.emit("import-progress", json!({"stage": stage, "done": done, "total": total}));
}

fn extract_prefix(zip: &mut zip::ZipArchive<std::fs::File>, prefixes: &[String], dest: &Path) -> Res<()> {
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).ctx("Повреждённый архив")?;
        let name = entry.name().replace('\\', "/");
        let Some(rest) = prefixes.iter().find_map(|p| name.strip_prefix(p.as_str())) else { continue };
        if rest.is_empty() || entry.is_dir() {
            continue;
        }
        let Some(out) = safe_join(dest, rest) else { continue };
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).ctx("Не удалось создать папку")?;
        }
        let mut f = std::fs::File::create(&out).ctx("Не удалось распаковать файл")?;
        std::io::copy(&mut entry, &mut f).ctx("Не удалось распаковать файл")?;
    }
    Ok(())
}

async fn import_mrpack<R: Runtime>(state: &AppState, app: &AppHandle<R>, path: &Path) -> Res<Instance> {
    let file = std::fs::File::open(path).ctx("Не удалось открыть файл")?;
    let mut zip = zip::ZipArchive::new(file).ctx("Это не архив сборки")?;
    let index: Value = {
        let mut entry = zip.by_name("modrinth.index.json").ctx("В архиве нет modrinth.index.json")?;
        let mut text = String::new();
        entry.read_to_string(&mut text).ctx("Не удалось прочитать сборку")?;
        serde_json::from_str(&text).ctx("Повреждённое описание сборки")?
    };
    let deps = index.get("dependencies").cloned().unwrap_or(Value::Null);
    let mc = deps.get("minecraft").and_then(|v| v.as_str()).ok_or("В сборке не указана версия Minecraft")?;
    let (loader, lv) = if let Some(v) = deps.get("fabric-loader").and_then(|v| v.as_str()) {
        ("fabric", Some(v.to_string()))
    } else if let Some(v) = deps.get("quilt-loader").and_then(|v| v.as_str()) {
        ("quilt", Some(v.to_string()))
    } else if let Some(v) = deps.get("neoforge").and_then(|v| v.as_str()) {
        ("neoforge", Some(v.to_string()))
    } else if let Some(v) = deps.get("forge").and_then(|v| v.as_str()) {
        ("forge", Some(format!("{mc}-{v}")))
    } else {
        ("vanilla", None)
    };
    let name = index.get("name").and_then(|v| v.as_str()).unwrap_or("Импортированная сборка");
    let inst = new_instance(state, name, mc, loader, lv)?;
    let game = instances::game_dir(state, &inst.id);
    let mut dl = Vec::new();
    for f in index.get("files").and_then(|f| f.as_array()).into_iter().flatten() {
        if f.pointer("/env/client").and_then(|v| v.as_str()) == Some("unsupported") {
            continue;
        }
        let Some(rel) = f.get("path").and_then(|v| v.as_str()) else { continue };
        let Some(dest) = safe_join(&game, rel) else { continue };
        let Some(url) = f.pointer("/downloads/0").and_then(|v| v.as_str()) else { continue };
        dl.push(Download {
            url: url.to_string(),
            path: dest,
            sha1: f.pointer("/hashes/sha1").and_then(|v| v.as_str()).map(String::from),
            size: f.get("fileSize").and_then(|v| v.as_u64()),
        });
    }
    let app2 = app.clone();
    download_all(&state.http, dl, move |d, t| emit(&app2, "files", d, t)).await?;
    emit(app, "overrides", 0, 1);
    extract_prefix(&mut zip, &["overrides/".into(), "client-overrides/".into()], &game)?;
    Ok(inst)
}

fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut e = zip.by_name(name).ok()?;
    let mut s = String::new();
    e.read_to_string(&mut s).ok()?;
    Some(s)
}

fn import_prism<R: Runtime>(state: &AppState, app: &AppHandle<R>, path: &Path) -> Res<Instance> {
    let file = std::fs::File::open(path).ctx("Не удалось открыть файл")?;
    let mut zip = zip::ZipArchive::new(file).ctx("Это не архив сборки")?;
    let pack_name = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|e| e.name().replace('\\', "/")))
        .find(|n| n.ends_with("mmc-pack.json"))
        .ok_or("В архиве нет mmc-pack.json — это не сборка Prism/MultiMC")?;
    let prefix = pack_name.trim_end_matches("mmc-pack.json").to_string();
    let pack: Value = serde_json::from_str(&read_entry(&mut zip, &pack_name).unwrap_or_default()).ctx("Повреждённый mmc-pack.json")?;
    let cfg = read_entry(&mut zip, &format!("{prefix}instance.cfg")).unwrap_or_default();
    let name = cfg
        .lines()
        .find_map(|l| l.strip_prefix("name="))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Сборка из Prism".into());
    let mut mc = None;
    let mut loader = ("vanilla", None);
    for c in pack.get("components").and_then(|c| c.as_array()).into_iter().flatten() {
        let uid = c.get("uid").and_then(|v| v.as_str()).unwrap_or("");
        let ver = c.get("version").and_then(|v| v.as_str()).map(String::from);
        match uid {
            "net.minecraft" => mc = ver,
            "net.fabricmc.fabric-loader" => loader = ("fabric", ver),
            "org.quiltmc.quilt-loader" => loader = ("quilt", ver),
            "net.neoforged" => loader = ("neoforge", ver),
            "net.minecraftforge" => loader = ("forge", ver),
            _ => {}
        }
    }
    let mc = mc.ok_or("В сборке не указана версия Minecraft")?;
    let lv = match (loader.0, loader.1) {
        ("forge", Some(v)) => Some(format!("{mc}-{v}")),
        (_, v) => v,
    };
    let inst = new_instance(state, &name, &mc, loader.0, lv)?;
    emit(app, "overrides", 0, 1);
    extract_prefix(
        &mut zip,
        &[format!("{prefix}.minecraft/"), format!("{prefix}minecraft/")],
        &instances::game_dir(state, &inst.id),
    )?;
    Ok(inst)
}

#[tauri::command]
pub async fn import_pack<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, path: String) -> Res<Instance> {
    let p = PathBuf::from(&path);
    let file = std::fs::File::open(&p).ctx("Не удалось открыть файл")?;
    let zip = zip::ZipArchive::new(file).ctx("Это не архив сборки")?;
    let is_mrpack = zip.file_names().any(|n| n == "modrinth.index.json");
    drop(zip);
    if is_mrpack {
        import_mrpack(&state, &app, &p).await
    } else {
        import_prism(&state, &app, &p)
    }
}

fn add_dir(zip: &mut zip::ZipWriter<std::fs::File>, dir: &Path, prefix: &str) -> Res<()> {
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = e.path();
        let name = format!("{prefix}{}", e.file_name().to_string_lossy());
        if path.is_dir() {
            add_dir(zip, &path, &format!("{name}/"))?;
        } else {
            zip.start_file(name, opts).ctx("Не удалось записать архив")?;
            zip.write_all(&std::fs::read(&path).ctx("Не удалось прочитать файл")?).ctx("Не удалось записать архив")?;
        }
    }
    Ok(())
}

/// Exports a build as .mrpack: files known to Modrinth become download links, the rest goes in overrides.
#[tauri::command]
pub async fn export_pack(state: State<'_, AppState>, instance_id: String, dest: String) -> Res<()> {
    let inst = instances::load(&state, &instance_id)?;
    let game = instances::game_dir(&state, &instance_id);
    let mut local = Vec::new();
    for kind in ["mod", "resourcepack", "shader"] {
        let f = folder(kind)?;
        for e in std::fs::read_dir(game.join(f)).into_iter().flatten().flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if path.is_file() && (name.ends_with(".jar") || name.ends_with(".zip")) {
                if let Some(h) = sha1_file(&path) {
                    local.push((format!("{f}/{name}"), path, h));
                }
            }
        }
    }
    let hashes: Vec<&String> = local.iter().map(|(_, _, h)| h).collect();
    let found: Value = if hashes.is_empty() {
        Value::Null
    } else {
        state
            .http
            .post(format!("{API}/version_files"))
            .json(&json!({"hashes": hashes, "algorithm": "sha1"}))
            .send()
            .await
            .ctx("Нет соединения с Modrinth")?
            .json()
            .await
            .unwrap_or(Value::Null)
    };
    let mut files = Vec::new();
    let mut overrides = Vec::new();
    for (rel, path, h) in &local {
        let url = found.get(h).and_then(|v| v.get("files")).and_then(|f| f.as_array()).and_then(|a| {
            a.iter()
                .find(|f| f.pointer("/hashes/sha1").and_then(|x| x.as_str()) == Some(h.as_str()))
                .and_then(|f| f.get("url").and_then(|u| u.as_str()))
        });
        match url {
            Some(url) => {
                let bytes = std::fs::read(path).ctx("Не удалось прочитать файл")?;
                files.push(json!({
                    "path": rel,
                    "hashes": {"sha1": h, "sha512": hex::encode(Sha512::digest(&bytes))},
                    "env": {"client": "required", "server": if rel.starts_with("mods/") { "optional" } else { "unsupported" }},
                    "downloads": [url],
                    "fileSize": bytes.len()
                }));
            }
            None => overrides.push((rel.clone(), path.clone())),
        }
    }
    let mut deps = json!({"minecraft": inst.mc_version});
    match inst.loader.as_str() {
        "fabric" | "quilt" => {
            let v = match &inst.loader_version {
                Some(v) => v.clone(),
                None => default_loader(&state, &inst.loader, &inst.mc_version).await?,
            };
            deps[format!("{}-loader", inst.loader)] = json!(v);
        }
        "neoforge" => {
            if let Some(v) = &inst.loader_version {
                deps["neoforge"] = json!(v);
            }
        }
        "forge" => {
            if let Some(v) = &inst.loader_version {
                let short = v.strip_prefix(&format!("{}-", inst.mc_version)).unwrap_or(v);
                deps["forge"] = json!(short);
            }
        }
        _ => {}
    }
    let index = json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.0.0",
        "name": inst.name,
        "summary": "Exported from AniLauncher",
        "files": files,
        "dependencies": deps,
    });
    let out = std::fs::File::create(&dest).ctx("Не удалось создать файл")?;
    let mut zip = zip::ZipWriter::new(out);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("modrinth.index.json", opts).ctx("Не удалось записать архив")?;
    zip.write_all(serde_json::to_string_pretty(&index).unwrap_or_default().as_bytes())
        .ctx("Не удалось записать архив")?;
    for (rel, path) in overrides {
        zip.start_file(format!("overrides/{rel}"), opts).ctx("Не удалось записать архив")?;
        zip.write_all(&std::fs::read(&path).ctx("Не удалось прочитать файл")?).ctx("Не удалось записать архив")?;
    }
    for d in ["config", "defaultconfigs", "kubejs"] {
        let dir = game.join(d);
        if dir.is_dir() {
            add_dir(&mut zip, &dir, &format!("overrides/{d}/"))?;
        }
    }
    zip.finish().ctx("Не удалось записать архив")?;
    Ok(())
}

/// Installs a Modrinth modpack as a new build (with its gallery picture as the build art).
#[tauri::command]
pub async fn install_modpack<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    project_id: String,
    version_id: Option<String>,
) -> Res<Instance> {
    let version: Value = match version_id {
        Some(v) => get_json(&state.http, &format!("{API}/version/{v}")).await?,
        None => {
            let list: Vec<Value> = get_json(&state.http, &format!("{API}/project/{project_id}/version")).await?;
            list.iter()
                .find(|v| v.get("version_type").and_then(|t| t.as_str()) == Some("release"))
                .or_else(|| list.first())
                .cloned()
                .ok_or("У модпака нет версий")?
        }
    };
    let file = version
        .get("files")
        .and_then(|f| f.as_array())
        .and_then(|a| {
            a.iter()
                .find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
                .or_else(|| a.first())
        })
        .ok_or("У модпака нет файла")?;
    let url = file.get("url").and_then(|u| u.as_str()).ok_or("Нет ссылки на модпак")?;
    let dest = state
        .root
        .join("cache")
        .join(format!("modpack-{}.mrpack", uuid::Uuid::new_v4().simple()));
    emit(&app, "download", 0, 1);
    download_all(
        &state.http,
        vec![Download {
            url: url.to_string(),
            path: dest.clone(),
            sha1: file.pointer("/hashes/sha1").and_then(|h| h.as_str()).map(String::from),
            size: file.get("size").and_then(|s| s.as_u64()),
        }],
        |_, _| {},
    )
    .await?;
    let result = import_mrpack(&state, &app, &dest).await;
    let _ = std::fs::remove_file(&dest);
    let mut inst = result?;
    if let Ok(project) = get_json::<Value>(&state.http, &format!("{API}/project/{project_id}")).await {
        let gallery = project.get("gallery").and_then(|g| g.as_array());
        let image = gallery
            .and_then(|g| {
                g.iter()
                    .find(|x| x.get("featured").and_then(|f| f.as_bool()).unwrap_or(false))
                    .or_else(|| g.first())
            })
            .and_then(|x| x.get("raw_url").or_else(|| x.get("url")).and_then(|u| u.as_str()))
            .map(String::from);
        if let Some(img) = image {
            inst.art = Some(crate::instances::Art { kind: "url".into(), value: img });
        }
        if let Some(title) = project.get("title").and_then(|t| t.as_str()) {
            inst.name = title.chars().take(48).collect();
        }
        instances::store(&state, &inst)?;
    }
    Ok(inst)
}
