//! "Like my friend's": turn links (Modrinth, CurseForge, YouTube descriptions, collections)
//! or a plain list of mod names into Modrinth projects. Also: files dropped onto the launcher.

use crate::content::{modrinth_loaders, sha1_file, API};
use crate::instances;
use crate::net::get_json;
use crate::state::AppState;
use crate::util::{Ctx, Res};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    project_id: String,
    slug: String,
    title: String,
    icon: Option<String>,
    /// mod | modpack | resourcepack | shader | datapack | plugin
    project_type: String,
    /// what in the input pointed to it
    source: String,
    /// false when the project has no release for this Minecraft version and loader
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    items: Vec<Found>,
    not_found: Vec<String>,
}

/// Text of a YouTube video description (from the watch page).
async fn youtube_description(state: &AppState, url: &str) -> Option<String> {
    let html = state
        .http
        .get(url)
        .header("Accept-Language", "en")
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let key = "\"shortDescription\":\"";
    let start = html.find(key)? + key.len() - 1;
    let bytes = html.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            break;
        }
        i += 1;
    }
    serde_json::from_str::<String>(&html[start..=i.min(html.len() - 1)]).ok()
}

/// Collects Modrinth slugs/ids from links and the names of everything else.
async fn collect(state: &AppState, text: &str, depth: u8, slugs: &mut Vec<(String, String)>, names: &mut Vec<String>) {
    let mr = regex::Regex::new(r"modrinth\.com/(mod|modpack|resourcepack|shader|plugin|datapack|project)/([A-Za-z0-9_.\-]+)").unwrap();
    let col = regex::Regex::new(r"modrinth\.com/collection/([A-Za-z0-9]+)").unwrap();
    let cf = regex::Regex::new(r"curseforge\.com/minecraft/(?:mc-mods|modpacks|texture-packs|shaders|customization)/([A-Za-z0-9_\-]+)").unwrap();
    let yt = regex::Regex::new(r"(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/)([A-Za-z0-9_\-]{6,})").unwrap();
    for c in mr.captures_iter(text) {
        slugs.push((c[2].to_string(), c[0].to_string()));
    }
    for c in cf.captures_iter(text) {
        // CurseForge files can't be downloaded without their key; most mods use the same slug on Modrinth
        slugs.push((c[1].to_string(), c[0].to_string()));
    }
    for c in col.captures_iter(text) {
        if let Ok(v) = get_json::<Value>(&state.http, &format!("https://api.modrinth.com/v3/collection/{}", &c[1])).await {
            for p in v.get("projects").and_then(|p| p.as_array()).into_iter().flatten() {
                if let Some(id) = p.as_str() {
                    slugs.push((id.to_string(), c[0].to_string()));
                }
            }
        }
    }
    if depth == 0 {
        for c in yt.captures_iter(text) {
            let url = format!("https://www.youtube.com/watch?v={}", &c[1]);
            if let Some(desc) = youtube_description(state, &url).await {
                Box::pin(collect(state, &desc, depth + 1, slugs, names)).await;
            }
        }
    }
    // Lines without any link are treated as mod names ("Sodium", "- Iris Shaders", "3. JEI")
    if depth == 0 {
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.ends_with(':') || l.contains("://") || l.contains(".com/") || l.contains("youtu") {
                continue;
            }
            let name = l
                .trim_start_matches(|c: char| c.is_ascii_digit() || "-*•.)#· ".contains(c))
                .split(" - ")
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if name.len() >= 2 && name.len() <= 60 {
                names.push(name);
            }
        }
    }
}

fn found_from(p: &Value, source: &str, target: Option<(&str, &str)>) -> Found {
    let s = |k: &str| p.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let has = |k: &str, want: &str| p.get(k).and_then(|x| x.as_array()).map(|a| a.iter().any(|v| v.as_str() == Some(want))).unwrap_or(true);
    let available = match target {
        Some((mc, loader)) if !mc.is_empty() => {
            let game_ok = has("game_versions", mc);
            let ls = modrinth_loaders(loader);
            let loader_ok = s("project_type") != "mod" || ls.is_empty() || ls.iter().any(|l| has("loaders", l));
            game_ok && loader_ok
        }
        _ => true,
    };
    Found {
        available,
        project_id: s("id"),
        slug: s("slug"),
        title: s("title"),
        icon: p.get("icon_url").and_then(|x| x.as_str()).map(String::from),
        project_type: s("project_type"),
        source: source.to_string(),
    }
}

#[tauri::command]
pub async fn resolve_share(state: State<'_, AppState>, text: String, loader: Option<String>, mc: Option<String>) -> Res<Resolved> {
    let mut slugs = Vec::new();
    let mut names = Vec::new();
    collect(&state, &text, 0, &mut slugs, &mut names).await;
    let mut items: Vec<Found> = Vec::new();
    let mut not_found = Vec::new();
    let mut seen = HashSet::new();
    if !slugs.is_empty() {
        let ids: Vec<&str> = slugs.iter().map(|(s, _)| s.as_str()).collect();
        let list: Vec<Value> = state
            .http
            .get(format!("{API}/projects"))
            .query(&[("ids", serde_json::to_string(&ids).unwrap_or_default())])
            .send()
            .await
            .ctx("Нет соединения с Modrinth")?
            .json()
            .await
            .unwrap_or_default();
        for (slug, source) in &slugs {
            match list.iter().find(|p| {
                p.get("slug").and_then(|x| x.as_str()) == Some(slug.as_str()) || p.get("id").and_then(|x| x.as_str()) == Some(slug.as_str())
            }) {
                Some(p) => {
                    let f = found_from(p, source, mc.as_deref().zip(loader.as_deref()));
                    if seen.insert(f.project_id.clone()) {
                        items.push(f);
                    }
                }
                None => not_found.push(source.clone()),
            }
        }
    }
    for name in names {
        let mut facets = vec![vec!["project_type:mod".to_string()]];
        if let Some(l) = loader.as_ref() {
            let ls = modrinth_loaders(l);
            if !ls.is_empty() {
                facets.push(ls.iter().map(|x| format!("categories:{x}")).collect());
            }
        }
        if let Some(v) = mc.as_ref().filter(|v| !v.is_empty()) {
            facets.push(vec![format!("versions:{v}")]);
        }
        let r: Value = state
            .http
            .get(format!("{API}/search"))
            .query(&[("query", name.as_str()), ("limit", "1"), ("facets", &serde_json::to_string(&facets).unwrap_or_default())])
            .send()
            .await
            .ctx("Нет соединения с Modrinth")?
            .json()
            .await
            .unwrap_or(Value::Null);
        match r.pointer("/hits/0") {
            Some(h) => {
                let f = Found {
                    project_id: h.get("project_id").and_then(|x| x.as_str()).unwrap_or("").into(),
                    slug: h.get("slug").and_then(|x| x.as_str()).unwrap_or("").into(),
                    title: h.get("title").and_then(|x| x.as_str()).unwrap_or("").into(),
                    icon: h.get("icon_url").and_then(|x| x.as_str()).filter(|x| !x.is_empty()).map(String::from),
                    project_type: "mod".into(),
                    source: name.clone(),
                    // the search was already limited to this version and loader
                    available: true,
                };
                if seen.insert(f.project_id.clone()) {
                    items.push(f);
                }
            }
            None => not_found.push(name),
        }
    }
    Ok(Resolved { items, not_found })
}

// ---------- dropped files ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DropInfo {
    path: String,
    name: String,
    /// mod | resourcepack | shader | world | modpack | unknown
    kind: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
    /// builds that fit, best first
    suggested: Vec<String>,
}

fn zip_names(path: &Path) -> Vec<String> {
    std::fs::File::open(path)
        .ok()
        .and_then(|f| zip::ZipArchive::new(f).ok())
        .map(|z| z.file_names().map(String::from).collect())
        .unwrap_or_default()
}

fn read_entry(path: &Path, name: &str) -> Option<String> {
    let f = std::fs::File::open(path).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let mut e = z.by_name(name).ok()?;
    let mut s = String::new();
    e.read_to_string(&mut s).ok()?;
    Some(s)
}

#[tauri::command]
pub async fn analyze_drop(state: State<'_, AppState>, path: String) -> Res<DropInfo> {
    let p = PathBuf::from(&path);
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let lower = name.to_lowercase();
    let entries = zip_names(&p);
    let mut kind = "unknown".to_string();
    let mut loaders: Vec<String> = Vec::new();
    let mut games: Vec<String> = Vec::new();
    if lower.ends_with(".mrpack") || entries.iter().any(|e| e == "modrinth.index.json" || e.ends_with("mmc-pack.json")) {
        kind = "modpack".into();
    } else if entries.iter().any(|e| e.ends_with("level.dat")) {
        kind = "world".into();
    } else if lower.ends_with(".jar") {
        kind = "mod".into();
        if entries.iter().any(|e| e == "fabric.mod.json") {
            loaders.push("fabric".into());
            if let Some(v) = read_entry(&p, "fabric.mod.json").and_then(|t| serde_json::from_str::<Value>(&t.replace(['\n', '\r', '\t'], " ")).ok()) {
                if let Some(mc) = v.pointer("/depends/minecraft") {
                    games.push(match mc {
                        Value::String(s) => s.clone(),
                        Value::Array(a) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" | "),
                        _ => String::new(),
                    });
                }
            }
        }
        if entries.iter().any(|e| e == "quilt.mod.json") {
            loaders.push("quilt".into());
        }
        if entries.iter().any(|e| e == "META-INF/neoforge.mods.toml") {
            loaders.push("neoforge".into());
        }
        if entries.iter().any(|e| e == "META-INF/mods.toml") {
            loaders.push("forge".into());
        }
        // Modrinth knows exact game versions when the file is published there
        if let Some(h) = sha1_file(&p) {
            if let Ok(r) = state.http.get(format!("{API}/version_file/{h}")).query(&[("algorithm", "sha1")]).send().await {
                if r.status().is_success() {
                    if let Ok(v) = r.json::<Value>().await {
                        let arr = |k: &str| -> Vec<String> {
                            v.get(k).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
                        };
                        loaders = arr("loaders");
                        games = arr("game_versions");
                    }
                }
            }
        }
    } else if entries.iter().any(|e| e.starts_with("shaders/") || e.contains("/shaders/")) {
        kind = "shader".into();
    } else if entries.iter().any(|e| e == "pack.mcmeta" || e.ends_with("/pack.mcmeta")) {
        kind = "resourcepack".into();
    }
    // Which builds fit
    let mut list = Vec::new();
    for e in std::fs::read_dir(state.instances()).into_iter().flatten().flatten() {
        if let Ok(inst) = instances::load(&state, &e.file_name().to_string_lossy()) {
            list.push(inst);
        }
    }
    list.sort_by(|a, b| b.last_played.unwrap_or(b.created).cmp(&a.last_played.unwrap_or(a.created)));
    let suggested = list
        .iter()
        .filter(|i| match kind.as_str() {
            "mod" => {
                let ok_loader = loaders.is_empty() || modrinth_loaders(&i.loader).iter().any(|l| loaders.iter().any(|x| x == l));
                let exact = games.iter().any(|g| g == &i.mc_version);
                let ranged = games.iter().any(|g| crate::conflicts::matches(&i.mc_version, &Value::String(g.clone())));
                ok_loader && i.loader != "vanilla" && (games.is_empty() || exact || ranged)
            }
            _ => true,
        })
        .map(|i| i.id.clone())
        .collect();
    Ok(DropInfo { path, name, kind, loaders, game_versions: games, suggested })
}

/// Puts a dropped file where it belongs inside a build.
#[tauri::command]
pub fn place_drop(state: State<'_, AppState>, path: String, instance_id: String, kind: String) -> Res<String> {
    let src = PathBuf::from(&path);
    let name = src.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("Неверный файл")?;
    let game = instances::game_dir(&state, &instance_id);
    match kind.as_str() {
        "mod" | "resourcepack" | "shader" => {
            let folder = crate::content::folder(&kind)?;
            let dir = game.join(folder);
            std::fs::create_dir_all(&dir).ctx("Не удалось создать папку")?;
            std::fs::copy(&src, dir.join(&name)).ctx("Не удалось скопировать файл")?;
            Ok(folder.to_string())
        }
        "world" => {
            let f = std::fs::File::open(&src).ctx("Не удалось открыть архив")?;
            let mut zip = zip::ZipArchive::new(f).ctx("Повреждённый архив")?;
            let level = zip
                .file_names()
                .find(|n| n.ends_with("level.dat"))
                .map(String::from)
                .ok_or("В архиве нет мира")?;
            let prefix = level.trim_end_matches("level.dat").to_string();
            let base = prefix
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .map(String::from)
                .unwrap_or_else(|| name.trim_end_matches(".zip").to_string());
            let saves = game.join("saves");
            let mut target = saves.join(&base);
            let mut n = 2;
            while target.exists() {
                target = saves.join(format!("{base} ({n})"));
                n += 1;
            }
            for i in 0..zip.len() {
                let mut e = zip.by_index(i).ctx("Повреждённый архив")?;
                let entry = e.name().replace('\\', "/");
                let Some(rest) = entry.strip_prefix(&prefix) else { continue };
                if rest.is_empty() || e.is_dir() || rest.contains("..") {
                    continue;
                }
                let out = target.join(rest);
                if let Some(p) = out.parent() {
                    std::fs::create_dir_all(p).ctx("Не удалось распаковать мир")?;
                }
                let mut w = std::fs::File::create(&out).ctx("Не удалось распаковать мир")?;
                std::io::copy(&mut e, &mut w).ctx("Не удалось распаковать мир")?;
            }
            Ok(target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        }
        _ => Err("Не знаю, куда положить этот файл".into()),
    }
}

// ---------- images for the player card ----------

fn png_bytes(data: &str) -> Res<Vec<u8>> {
    use base64::Engine;
    let raw = data.split_once(',').map(|(_, b)| b).unwrap_or(data);
    base64::engine::general_purpose::STANDARD.decode(raw).ctx("Неверная картинка")
}

#[tauri::command]
pub fn save_png(path: String, data: String) -> Res<()> {
    if !path.to_lowercase().ends_with(".png") {
        return Err("Сохранять можно только PNG".into());
    }
    std::fs::write(&path, png_bytes(&data)?).ctx("Не удалось сохранить файл")
}

#[tauri::command]
pub fn copy_png<R: tauri::Runtime>(app: tauri::AppHandle<R>, data: String) -> Res<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let img = tauri::image::Image::from_bytes(&png_bytes(&data)?).ctx("Неверная картинка")?;
    app.clipboard().write_image(&img).ctx("Не удалось скопировать")
}
