//! Mods, resource packs and shaders from Modrinth: search, install with dependencies, updates.

use crate::instances;
use crate::net::get_json;
use crate::state::AppState;
use crate::util::{read_json, write_json, Ctx, Res};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::State;

pub const API: &str = "https://api.modrinth.com/v2";

pub fn folder(kind: &str) -> Res<&'static str> {
    match kind {
        "mod" => Ok("mods"),
        "resourcepack" => Ok("resourcepacks"),
        "shader" => Ok("shaderpacks"),
        _ => Err("Неизвестный тип контента".into()),
    }
}

/// Loader names Modrinth understands for a build's loader (Quilt also runs Fabric mods).
pub fn modrinth_loaders(loader: &str) -> Vec<&'static str> {
    match loader {
        "fabric" => vec!["fabric"],
        "quilt" => vec!["quilt", "fabric"],
        "forge" => vec!["forge"],
        "neoforge" => vec!["neoforge"],
        _ => vec![],
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub filename: String,
    pub kind: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub version_number: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Meta {
    pub entries: Vec<Entry>,
}

fn meta_path(state: &AppState, id: &str) -> PathBuf {
    instances::dir(state, id).join("content.json")
}

pub fn load_meta(state: &AppState, id: &str) -> Meta {
    read_json(&meta_path(state, id)).unwrap_or_default()
}

pub fn save_meta(state: &AppState, id: &str, meta: &Meta) -> Res<()> {
    write_json(&meta_path(state, id), meta)
}

pub fn sha1_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(Sha1::digest(&bytes)))
}

fn safe_name(name: &str) -> Res<()> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Неверное имя файла".into());
    }
    Ok(())
}

// ---------- search ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    downloads: u64,
    author: String,
    categories: Vec<String>,
}

#[derive(Serialize)]
pub struct SearchResult {
    hits: Vec<Hit>,
    total: u64,
}

#[tauri::command]
pub async fn modrinth_search(
    state: State<'_, AppState>,
    query: String,
    kind: String,
    mc: Option<String>,
    loader: Option<String>,
    offset: u32,
    sort: Option<String>,
) -> Res<SearchResult> {
    if kind != "modpack" {
        folder(&kind)?;
    }
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{kind}")]];
    if let Some(mc) = mc.filter(|m| !m.is_empty()) {
        facets.push(vec![format!("versions:{mc}")]);
    }
    if kind == "mod" || kind == "modpack" {
        if let Some(l) = loader.filter(|l| !l.is_empty()) {
            let ls = modrinth_loaders(&l);
            if !ls.is_empty() {
                facets.push(ls.iter().map(|x| format!("categories:{x}")).collect());
            }
        }
    }
    let index = match sort.as_deref() {
        Some("downloads") | Some("follows") | Some("newest") | Some("updated") | Some("relevance") => sort.unwrap(),
        _ => "relevance".to_string(),
    };
    let resp: Value = state
        .http
        .get(format!("{API}/search"))
        .query(&[
            ("query", query.as_str()),
            ("facets", &serde_json::to_string(&facets).unwrap_or_default()),
            ("index", index.as_str()),
            ("offset", &offset.to_string()),
            ("limit", "24"),
        ])
        .send()
        .await
        .ctx("Нет соединения с Modrinth")?
        .json()
        .await
        .ctx("Неверный ответ Modrinth")?;
    let hits = resp
        .get("hits")
        .and_then(|h| h.as_array())
        .map(|a| {
            a.iter()
                .map(|h| Hit {
                    project_id: s(h, "project_id"),
                    slug: s(h, "slug"),
                    title: s(h, "title"),
                    description: s(h, "description"),
                    icon_url: h.get("icon_url").and_then(|v| v.as_str()).filter(|v| !v.is_empty()).map(String::from),
                    downloads: h.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                    author: s(h, "author"),
                    categories: h
                        .get("display_categories")
                        .and_then(|v| v.as_array())
                        .map(|c| c.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(SearchResult {
        hits,
        total: resp.get("total_hits").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    id: String,
    name: String,
    version_number: String,
    version_type: String,
    date: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
}

async fn compatible_versions(state: &AppState, project: &str, kind: &str, mc: &str, loader: &str) -> Res<Vec<Value>> {
    let mut query: Vec<(&str, String)> = vec![("game_versions", serde_json::to_string(&[mc]).unwrap_or_default())];
    if kind == "mod" {
        let ls = modrinth_loaders(loader);
        if !ls.is_empty() {
            query.push(("loaders", serde_json::to_string(&ls).unwrap_or_default()));
        }
    }
    let resp = state
        .http
        .get(format!("{API}/project/{project}/version"))
        .query(&query)
        .send()
        .await
        .ctx("Нет соединения с Modrinth")?;
    if !resp.status().is_success() {
        return Err(format!("Modrinth ответил {}", resp.status()));
    }
    resp.json::<Vec<Value>>().await.ctx("Неверный ответ Modrinth")
}

#[tauri::command]
pub async fn modrinth_versions(state: State<'_, AppState>, instance_id: String, project_id: String, kind: String) -> Res<Vec<VersionInfo>> {
    let inst = instances::load(&state, &instance_id)?;
    let list = compatible_versions(&state, &project_id, &kind, &inst.mc_version, &inst.loader).await?;
    Ok(list
        .iter()
        .map(|v| VersionInfo {
            id: s(v, "id"),
            name: s(v, "name"),
            version_number: s(v, "version_number"),
            version_type: s(v, "version_type"),
            date: s(v, "date_published"),
            game_versions: strs(v, "game_versions"),
            loaders: strs(v, "loaders"),
        })
        .collect())
}

fn strs(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn primary_file(version: &Value) -> Option<&Value> {
    let files = version.get("files")?.as_array()?;
    files
        .iter()
        .find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
        .or_else(|| files.first())
}

async fn project_info(state: &AppState, project: &str) -> Option<Value> {
    get_json::<Value>(&state.http, &format!("{API}/project/{project}")).await.ok()
}

/// Downloads one version's primary file into the build and records it. Returns the entry.
async fn install_version(state: &AppState, id: &str, kind: &str, version: &Value, meta: &mut Meta) -> Res<Entry> {
    let file = primary_file(version).ok_or("У этой версии нет файла")?;
    let filename = s(file, "filename");
    safe_name(&filename)?;
    let url = s(file, "url");
    let sha1 = file.pointer("/hashes/sha1").and_then(|v| v.as_str()).map(String::from);
    let dir = instances::game_dir(state, id).join(folder(kind)?);
    std::fs::create_dir_all(&dir).ctx("Не удалось создать папку")?;
    let bytes = state
        .http
        .get(&url)
        .send()
        .await
        .ctx("Нет соединения")?
        .bytes()
        .await
        .ctx("Не удалось скачать файл")?;
    if let Some(h) = &sha1 {
        if hex::encode(Sha1::digest(&bytes)) != *h {
            return Err("Файл скачался с ошибкой, попробуй ещё раз".into());
        }
    }
    std::fs::write(dir.join(&filename), &bytes).ctx("Не удалось сохранить файл")?;
    let project_id = s(version, "project_id");
    let info = project_info(state, &project_id).await;
    let entry = Entry {
        filename: filename.clone(),
        kind: kind.to_string(),
        project_id: Some(project_id.clone()),
        version_id: Some(s(version, "id")),
        version_number: Some(s(version, "version_number")),
        title: info.as_ref().map(|i| s(i, "title")).filter(|t| !t.is_empty()),
        icon: info
            .as_ref()
            .and_then(|i| i.get("icon_url").and_then(|v| v.as_str()).map(String::from)),
        sha1,
    };
    meta.entries.retain(|e| !(e.kind == kind && (e.filename == filename || e.project_id.as_deref() == Some(project_id.as_str()))));
    meta.entries.push(entry.clone());
    Ok(entry)
}

/// Installs a project (latest compatible version unless one is given) plus required dependencies.
#[tauri::command]
pub async fn install_content(
    state: State<'_, AppState>,
    instance_id: String,
    kind: String,
    project_id: String,
    version_id: Option<String>,
) -> Res<Vec<String>> {
    install_content_inner(&state, &instance_id, &kind, &project_id, version_id).await
}

pub async fn install_content_inner(
    state: &AppState,
    instance_id: &str,
    kind: &str,
    project_id: &str,
    version_id: Option<String>,
) -> Res<Vec<String>> {
    let (state, instance_id, kind, project_id) = (state, instance_id.to_string(), kind.to_string(), project_id.to_string());
    let inst = instances::load(state, &instance_id)?;
    let mut meta = load_meta(&state, &instance_id);
    let mut installed_titles = Vec::new();
    let mut seen: HashSet<String> = meta
        .entries
        .iter()
        .filter_map(|e| e.project_id.clone())
        .filter(|p| *p != project_id)
        .collect();
    let mut queue: Vec<(String, Option<String>)> = vec![(project_id.clone(), version_id)];
    while let Some((project, version)) = queue.pop() {
        if !seen.insert(project.clone()) {
            continue;
        }
        let v: Value = match version {
            Some(vid) => get_json(&state.http, &format!("{API}/version/{vid}")).await?,
            None => {
                let list = compatible_versions(&state, &project, &kind, &inst.mc_version, &inst.loader).await?;
                let pick = list
                    .iter()
                    .find(|v| s(v, "version_type") == "release")
                    .or_else(|| list.first())
                    .cloned();
                match pick {
                    Some(p) => p,
                    None if project == project_id => {
                        return Err(format!("Нет версии для Minecraft {} ({})", inst.mc_version, inst.loader));
                    }
                    None => continue,
                }
            }
        };
        // Remove an older copy of the same project first
        for old in meta.entries.iter().filter(|e| e.project_id.as_deref() == Some(project.as_str()) && e.kind == kind) {
            let dir = instances::game_dir(&state, &instance_id).join(folder(&kind)?);
            let _ = std::fs::remove_file(dir.join(&old.filename));
            let _ = std::fs::remove_file(dir.join(format!("{}.disabled", old.filename)));
        }
        let entry = install_version(&state, &instance_id, &kind, &v, &mut meta).await?;
        installed_titles.push(entry.title.unwrap_or(entry.filename));
        if kind == "mod" {
            for dep in v.get("dependencies").and_then(|d| d.as_array()).into_iter().flatten() {
                if s(dep, "dependency_type") != "required" {
                    continue;
                }
                let dep_version = dep.get("version_id").and_then(|x| x.as_str()).map(String::from);
                let dep_project = match dep.get("project_id").and_then(|x| x.as_str()) {
                    Some(p) => p.to_string(),
                    None => match &dep_version {
                        Some(vid) => {
                            let dv: Value = get_json(&state.http, &format!("{API}/version/{vid}")).await?;
                            s(&dv, "project_id")
                        }
                        None => continue,
                    },
                };
                if !seen.contains(&dep_project) {
                    queue.push((dep_project, dep_version));
                }
            }
        }
    }
    save_meta(&state, &instance_id, &meta)?;
    Ok(installed_titles)
}

// ---------- installed list ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    filename: String,
    enabled: bool,
    title: String,
    project_id: Option<String>,
    version_number: Option<String>,
    icon: Option<String>,
    size: u64,
}

/// Files in the build's folder, enriched with Modrinth info (looked up by hash for unknown files).
#[tauri::command]
pub async fn list_content(state: State<'_, AppState>, instance_id: String, kind: String) -> Res<Vec<Installed>> {
    let dir = instances::game_dir(&state, &instance_id).join(folder(&kind)?);
    let mut meta = load_meta(&state, &instance_id);
    let mut files = Vec::new();
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let path = e.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else { continue };
        let is_file = path.is_file();
        let ok_ext = name.ends_with(".jar") || name.ends_with(".zip") || name.ends_with(".disabled");
        if !(ok_ext && is_file) && !(kind != "mod" && path.is_dir()) {
            continue;
        }
        let enabled = !name.ends_with(".disabled");
        let base = name.trim_end_matches(".disabled").to_string();
        files.push((base, enabled, path));
    }
    // Look up unknown files on Modrinth by hash
    let mut unknown: HashMap<String, String> = HashMap::new();
    for (base, _, path) in &files {
        let known = meta.entries.iter().any(|e| e.kind == kind && e.filename == *base);
        if !known && path.is_file() {
            if let Some(h) = sha1_file(path) {
                unknown.insert(h, base.clone());
            }
        }
    }
    if !unknown.is_empty() {
        let hashes: Vec<&String> = unknown.keys().collect();
        if let Ok(resp) = state
            .http
            .post(format!("{API}/version_files"))
            .json(&json!({"hashes": hashes, "algorithm": "sha1"}))
            .send()
            .await
        {
            let found: Value = resp.json().await.unwrap_or(Value::Null);
            let mut projects = Vec::new();
            for (hash, base) in &unknown {
                if let Some(v) = found.get(hash) {
                    projects.push(s(v, "project_id"));
                    meta.entries.push(Entry {
                        filename: base.clone(),
                        kind: kind.clone(),
                        project_id: Some(s(v, "project_id")),
                        version_id: Some(s(v, "id")),
                        version_number: Some(s(v, "version_number")),
                        title: None,
                        icon: None,
                        sha1: Some(hash.clone()),
                    });
                } else {
                    meta.entries.push(Entry {
                        filename: base.clone(),
                        kind: kind.clone(),
                        sha1: Some(hash.clone()),
                        ..Default::default()
                    });
                }
            }
            if !projects.is_empty() {
                let ids = serde_json::to_string(&projects).unwrap_or_default();
                if let Ok(r) = state.http.get(format!("{API}/projects")).query(&[("ids", ids)]).send().await {
                    let list: Vec<Value> = r.json().await.unwrap_or_default();
                    for p in list {
                        let pid = s(&p, "id");
                        for e in meta.entries.iter_mut().filter(|e| e.project_id.as_deref() == Some(pid.as_str())) {
                            e.title = Some(s(&p, "title"));
                            e.icon = p.get("icon_url").and_then(|v| v.as_str()).map(String::from);
                        }
                    }
                }
            }
        }
        let _ = save_meta(&state, &instance_id, &meta);
    }
    let mut out: Vec<Installed> = files
        .into_iter()
        .map(|(base, enabled, path)| {
            let e = meta.entries.iter().find(|e| e.kind == kind && e.filename == base);
            Installed {
                title: e.and_then(|e| e.title.clone()).unwrap_or_else(|| base.trim_end_matches(".jar").trim_end_matches(".zip").to_string()),
                project_id: e.and_then(|e| e.project_id.clone()),
                version_number: e.and_then(|e| e.version_number.clone()),
                icon: e.and_then(|e| e.icon.clone()),
                size: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                filename: base,
                enabled,
            }
        })
        .collect();
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(out)
}

#[tauri::command(async)]
pub fn toggle_content(state: State<'_, AppState>, instance_id: String, kind: String, filename: String, enabled: bool) -> Res<()> {
    safe_name(&filename)?;
    let dir = instances::game_dir(&state, &instance_id).join(folder(&kind)?);
    let on = dir.join(&filename);
    let off = dir.join(format!("{filename}.disabled"));
    if enabled && off.exists() {
        std::fs::rename(&off, &on).ctx("Не удалось включить")?;
    } else if !enabled && on.exists() {
        std::fs::rename(&on, &off).ctx("Не удалось выключить")?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn remove_content(state: State<'_, AppState>, instance_id: String, kind: String, filename: String) -> Res<()> {
    safe_name(&filename)?;
    let dir = instances::game_dir(&state, &instance_id).join(folder(&kind)?);
    for p in [dir.join(&filename), dir.join(format!("{filename}.disabled"))] {
        if p.is_dir() {
            std::fs::remove_dir_all(&p).ctx("Не удалось удалить")?;
        } else if p.exists() {
            std::fs::remove_file(&p).ctx("Не удалось удалить")?;
        }
    }
    let mut meta = load_meta(&state, &instance_id);
    meta.entries.retain(|e| !(e.kind == kind && e.filename == filename));
    save_meta(&state, &instance_id, &meta)
}

// ---------- updates ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    kind: String,
    filename: String,
    title: String,
    from: String,
    to: String,
    version_id: String,
}

pub async fn find_updates(state: &AppState, instance_id: &str) -> Res<Vec<Update>> {
    let inst = instances::load(state, instance_id)?;
    let meta = load_meta(state, instance_id);
    let mut out = Vec::new();
    for kind in ["mod", "resourcepack", "shader"] {
        let dir = instances::game_dir(state, instance_id).join(folder(kind)?);
        let entries: Vec<&Entry> = meta
            .entries
            .iter()
            .filter(|e| e.kind == kind && e.project_id.is_some() && dir.join(&e.filename).exists())
            .collect();
        let mut by_hash = HashMap::new();
        for e in &entries {
            let h = e.sha1.clone().or_else(|| sha1_file(&dir.join(&e.filename)));
            if let Some(h) = h {
                by_hash.insert(h, *e);
            }
        }
        if by_hash.is_empty() {
            continue;
        }
        let mut body = json!({
            "hashes": by_hash.keys().collect::<Vec<_>>(),
            "algorithm": "sha1",
            "game_versions": [inst.mc_version],
        });
        if kind == "mod" {
            body["loaders"] = json!(modrinth_loaders(&inst.loader));
        }
        let resp: Value = match state.http.post(format!("{API}/version_files/update")).json(&body).send().await {
            Ok(r) => r.json().await.unwrap_or(Value::Null),
            Err(_) => continue,
        };
        for (hash, e) in &by_hash {
            if let Some(v) = resp.get(hash) {
                let new_id = s(v, "id");
                if Some(new_id.as_str()) != e.version_id.as_deref() && !new_id.is_empty() {
                    out.push(Update {
                        kind: kind.to_string(),
                        filename: e.filename.clone(),
                        title: e.title.clone().unwrap_or(e.filename.clone()),
                        from: e.version_number.clone().unwrap_or_default(),
                        to: s(v, "version_number"),
                        version_id: new_id,
                    });
                }
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn check_updates(state: State<'_, AppState>, instance_id: String) -> Res<Vec<Update>> {
    find_updates(&state, &instance_id).await
}

/// Installs the given updates (or all available ones when `only` is empty).
pub async fn apply_updates_inner(state: &AppState, instance_id: &str, only: &[String]) -> Res<usize> {
    let updates = find_updates(state, instance_id).await?;
    if updates.iter().any(|u| only.is_empty() || only.contains(&u.filename)) {
        crate::snapshots::auto(state, instance_id, "update", "");
    }
    let mut meta = load_meta(state, instance_id);
    let mut n = 0;
    for u in updates.iter().filter(|u| only.is_empty() || only.contains(&u.filename)) {
        let v: Value = get_json(&state.http, &format!("{API}/version/{}", u.version_id)).await?;
        let dir = instances::game_dir(state, instance_id).join(folder(&u.kind)?);
        let was_disabled = dir.join(format!("{}.disabled", u.filename)).exists();
        let entry = install_version(state, instance_id, &u.kind, &v, &mut meta).await?;
        if entry.filename != u.filename {
            let _ = std::fs::remove_file(dir.join(&u.filename));
            let _ = std::fs::remove_file(dir.join(format!("{}.disabled", u.filename)));
        }
        if was_disabled {
            let _ = std::fs::rename(dir.join(&entry.filename), dir.join(format!("{}.disabled", entry.filename)));
        }
        n += 1;
    }
    save_meta(state, instance_id, &meta)?;
    Ok(n)
}

#[tauri::command]
pub async fn apply_updates(state: State<'_, AppState>, instance_id: String, only: Vec<String>) -> Res<usize> {
    apply_updates_inner(&state, &instance_id, &only).await
}

/// One-click performance mods for a build. Each group installs the first project that fits.
#[derive(Serialize)]
pub struct OptimizeResult {
    installed: Vec<String>,
    skipped: Vec<String>,
}

#[tauri::command]
pub async fn optimize_instance(state: State<'_, AppState>, instance_id: String) -> Res<OptimizeResult> {
    let inst = instances::load(&state, &instance_id)?;
    let groups: Vec<Vec<&str>> = match inst.loader.as_str() {
        "fabric" | "quilt" => vec![
            vec!["sodium"],
            vec!["lithium"],
            vec!["ferrite-core"],
            vec!["immediatelyfast"],
            vec!["entityculling"],
            vec!["modernfix"],
            vec!["dynamic-fps"],
        ],
        "forge" | "neoforge" => vec![
            vec!["sodium", "embeddium"],
            vec!["ferrite-core"],
            vec!["modernfix"],
            vec!["entityculling"],
            vec!["immediatelyfast"],
        ],
        _ => return Err("Оптимизация работает в сборках с модами (Fabric, Quilt, Forge, NeoForge)".into()),
    };
    crate::snapshots::auto(&state, &instance_id, "optimize", "");
    let meta = load_meta(&state, &instance_id);
    let mut have: HashSet<String> = meta.entries.iter().filter_map(|e| e.project_id.clone()).collect();
    // Modrinth project ids for the slugs, so already-installed mods are recognised
    let slugs: Vec<&str> = groups.iter().flatten().copied().collect();
    let mut slug_to_id: HashMap<String, (String, String)> = HashMap::new();
    if let Ok(r) = state
        .http
        .get(format!("{API}/projects"))
        .query(&[("ids", serde_json::to_string(&slugs).unwrap_or_default())])
        .send()
        .await
    {
        let list: Vec<Value> = r.json().await.unwrap_or_default();
        for p in list {
            slug_to_id.insert(s(&p, "slug"), (s(&p, "id"), s(&p, "title")));
        }
    }
    let mut result = OptimizeResult { installed: vec![], skipped: vec![] };
    'groups: for group in groups {
        for slug in &group {
            if let Some((id, _)) = slug_to_id.get(*slug) {
                if have.contains(id) {
                    continue 'groups;
                }
            }
        }
        for slug in &group {
            match install_content_inner(&state, &instance_id, "mod", slug, None).await {
                Ok(titles) => {
                    if let Some((id, _)) = slug_to_id.get(*slug) {
                        have.insert(id.clone());
                    }
                    result.installed.extend(titles);
                    continue 'groups;
                }
                Err(_) => continue,
            }
        }
        let title = slug_to_id.get(group[0]).map(|(_, t)| t.clone()).unwrap_or(group[0].to_string());
        result.skipped.push(title);
    }
    Ok(result)
}
