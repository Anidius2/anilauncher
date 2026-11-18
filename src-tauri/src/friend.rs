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

/// "Sodium 0.6.13+mc1.21.11" -> "Sodium": a trailing version only spoils the search.
fn strip_version(name: &str) -> String {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1 {
        let last = words[words.len() - 1].trim_start_matches(['v', 'V']);
        let looks_like_version = last.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && last.contains('.');
        if !looks_like_version {
            break;
        }
        words.pop();
    }
    words.join(" ")
}

/// Collects Modrinth slugs/ids from links and the names of everything else.
async fn collect(state: &AppState, text: &str, depth: u8, slugs: &mut Vec<(String, String)>, names: &mut Vec<(String, Option<&'static str>)>) {
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
    // Lines without any link are treated as names ("Sodium", "- Iris Shaders", "3. JEI").
    // A heading like "Ресурспаки:" or "Shaders:" says what the names below it are.
    if depth == 0 {
        let mut hint: Option<&'static str> = None;
        for line in text.lines() {
            let l = line.trim();
            if l.ends_with(':') {
                hint = heading_kind(l);
                continue;
            }
            if l.is_empty() || l.contains("://") || l.contains(".com/") || l.contains("youtu") {
                continue;
            }
            // the header of a list exported by the launcher: "Build — Minecraft 1.21.11, Fabric — 63 mods"
            if l.contains(" — ") && l.contains("Minecraft ") {
                continue;
            }
            let name = l
                .trim_start_matches(|c: char| c.is_ascii_digit() || "-*•.)#· ".contains(c))
                .split(" - ")
                .next()
                .unwrap_or("")
                .split(" — ")
                .next()
                .unwrap_or("")
                .trim();
            let name = strip_version(name);
            if name.len() >= 2 && name.len() <= 60 {
                names.push((name, hint));
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

fn heading_kind(line: &str) -> Option<&'static str> {
    let l = line.to_lowercase();
    if l.contains("шейдер") || l.contains("shader") {
        Some("shader")
    } else if l.contains("ресурс") || l.contains("текстур") || l.contains("resource") || l.contains("texture") {
        Some("resourcepack")
    } else if l.contains("мод") || l.contains("mod") {
        Some("mod")
    } else {
        None
    }
}

async fn search_one(state: &AppState, name: &str, types: &[&str], loader: Option<&str>, mc: Option<&str>) -> Option<Value> {
    let mut facets = vec![types.iter().map(|t| format!("project_type:{t}")).collect::<Vec<_>>()];
    if types == ["mod"] {
        if let Some(l) = loader {
            let ls = modrinth_loaders(l);
            if !ls.is_empty() {
                facets.push(ls.iter().map(|x| format!("categories:{x}")).collect());
            }
        }
    }
    if let Some(v) = mc.filter(|v| !v.is_empty()) {
        facets.push(vec![format!("versions:{v}")]);
    }
    let r: Value = state
        .http
        .get(format!("{API}/search"))
        .query(&[("query", name), ("limit", "1"), ("facets", &serde_json::to_string(&facets).unwrap_or_default())])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    r.pointer("/hits/0").cloned()
}

/// "Interaction Preview", "interaction-preview", "interactionpreview" all become "interactionpreview".
fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// An own mod (from the launcher's catalog) that a name or a link slug points to.
fn own_match<'a>(mods: &'a [crate::own::OwnMod], what: &str) -> Option<&'a crate::own::OwnMod> {
    let w = squash(what);
    if w.len() < 4 {
        return None;
    }
    mods.iter()
        .find(|m| squash(&m.name) == w || squash(&m.id) == w)
        .or_else(|| {
            // "pr-plants-reimagined" ~ "plantsreimagined"; "Plants Reimagined" ~ "PR: Plants Reimagined"
            mods.iter().find(|m| {
                let (n, i) = (squash(&m.name), squash(&m.id));
                w.len() >= 6 && (n.contains(&w) || w.contains(&i) || w.contains(&n))
            })
        })
}

fn found_own(m: &crate::own::OwnMod, source: &str, target: Option<(&str, &str)>) -> Found {
    Found {
        available: match target {
            Some((mc, loader)) if !mc.is_empty() => crate::own::best(m, mc, loader).is_some(),
            _ => true,
        },
        project_id: format!("{}{}", crate::own::PREFIX, m.id),
        slug: m.id.clone(),
        title: m.name.clone(),
        icon: crate::own::icon_url(m),
        project_type: "mod".into(),
        source: source.to_string(),
    }
}

#[tauri::command]
pub async fn resolve_share(state: State<'_, AppState>, text: String, loader: Option<String>, mc: Option<String>) -> Res<Resolved> {
    let mut slugs = Vec::new();
    let mut names: Vec<(String, Option<&'static str>)> = Vec::new();
    collect(&state, &text, 0, &mut slugs, &mut names).await;
    let mut items: Vec<Found> = Vec::new();
    let mut not_found = Vec::new();
    let mut seen = HashSet::new();
    let own = crate::own::catalog(&state).await.mods;
    let target = mc.as_deref().zip(loader.as_deref());
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
                None => match own_match(&own, slug) {
                    // not on Modrinth (yet): maybe it's one of the author's own mods
                    Some(m) => {
                        let f = found_own(m, source, target);
                        if seen.insert(f.project_id.clone()) {
                            items.push(f);
                        }
                    }
                    None => not_found.push(source.clone()),
                },
            }
        }
    }
    for (name, hint) in names {
        if hint != Some("resourcepack") && hint != Some("shader") {
            if let Some(m) = own_match(&own, &name) {
                let f = found_own(m, &name, target);
                if seen.insert(f.project_id.clone()) {
                    items.push(f);
                }
                continue;
            }
        }
        let (l, v) = (loader.as_deref(), mc.as_deref());
        // Search the type the heading suggests first (mods when there is none); if that isn't an exact
        // title match, look among the other types too and keep whichever title fits the name better
        let want = squash(&name);
        let score = |h: &Option<Value>| -> u8 {
            let t = h.as_ref().and_then(|h| h.get("title")).and_then(|t| t.as_str()).map(squash).unwrap_or_default();
            if t.is_empty() { 0 } else if t == want { 3 } else if t.contains(&want) || want.contains(&t) { 2 } else { 1 }
        };
        let groups: Vec<Vec<&str>> = match hint {
            Some("resourcepack") => vec![vec!["resourcepack"], vec!["mod"], vec!["shader"]],
            Some("shader") => vec![vec!["shader"], vec!["mod"], vec!["resourcepack"]],
            _ => vec![vec!["mod"], vec!["resourcepack", "shader"]],
        };
        let mut hit: Option<Value> = None;
        for g in groups {
            let h = search_one(&state, &name, &g, l, v).await;
            if score(&h) > score(&hit) {
                hit = h;
            }
            if score(&hit) == 3 {
                break;
            }
        }
        match hit {
            Some(h) => {
                let kind = h.get("project_type").and_then(|x| x.as_str()).unwrap_or("mod").to_string();
                let f = Found {
                    project_id: h.get("project_id").and_then(|x| x.as_str()).unwrap_or("").into(),
                    slug: h.get("slug").and_then(|x| x.as_str()).unwrap_or("").into(),
                    title: h.get("title").and_then(|x| x.as_str()).unwrap_or("").into(),
                    icon: h.get("icon_url").and_then(|x| x.as_str()).filter(|x| !x.is_empty()).map(String::from),
                    project_type: kind,
                    source: name.clone(),
                    // the search was already limited to this version (and loader, for mods)
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

/// Resource pack or shader pack? Decided by what sits at the pack's own root (the archive root, or a single
/// wrapper folder): a resource pack has pack.mcmeta there; a shader pack has a shaders/ folder and no pack.mcmeta.
/// (Resource packs often carry assets/minecraft/shaders/ — that alone doesn't make them shader packs.)
pub fn pack_kind(entries: &[String]) -> Option<&'static str> {
    let names: Vec<String> = entries.iter().map(|e| e.replace('\\', "/").trim_start_matches("./").to_string()).collect();
    let mut roots: Vec<String> = vec![String::new()];
    // a single top folder wrapping everything ("MyPack/pack.mcmeta")
    let tops: std::collections::HashSet<&str> = names.iter().filter_map(|n| n.split('/').next()).filter(|t| !t.is_empty()).collect();
    if tops.len() == 1 {
        roots.push(format!("{}/", tops.iter().next().unwrap()));
    }
    for r in &roots {
        if names.iter().any(|n| *n == format!("{r}pack.mcmeta")) {
            return Some("resourcepack");
        }
    }
    for r in &roots {
        if names.iter().any(|n| n.starts_with(&format!("{r}shaders/"))) {
            return Some("shader");
        }
    }
    None
}

fn dir_names(path: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>, depth: u8) {
        if depth > 4 || out.len() > 4000 {
            return;
        }
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            if p.is_dir() {
                out.push(format!("{rel}/"));
                walk(base, &p, out, depth + 1);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(path, path, &mut out, 0);
    out
}

fn zip_names(path: &Path) -> Vec<String> {
    if path.is_dir() {
        return dir_names(path);
    }
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
    } else if let Some(k) = pack_kind(&entries) {
        kind = k.into();
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

fn copy_dir(from: &Path, to: &Path) -> Res<()> {
    std::fs::create_dir_all(to).ctx("Не удалось создать папку")?;
    for e in std::fs::read_dir(from).ctx("Не удалось открыть папку")?.flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t).ctx("Не удалось скопировать файл")?;
        }
    }
    Ok(())
}

/// Puts a dropped file where it belongs inside a build.
#[tauri::command(async)]
pub fn place_drop(state: State<'_, AppState>, path: String, instance_id: String, kind: String) -> Res<String> {
    let src = PathBuf::from(&path);
    let name = src.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("Неверный файл")?;
    let game = instances::game_dir(&state, &instance_id);
    match kind.as_str() {
        "mod" | "resourcepack" | "shader" => {
            let folder = crate::content::folder(&kind)?;
            let dir = game.join(folder);
            std::fs::create_dir_all(&dir).ctx("Не удалось создать папку")?;
            if src.is_dir() {
                copy_dir(&src, &dir.join(&name))?;
            } else {
                std::fs::copy(&src, dir.join(&name)).ctx("Не удалось скопировать файл")?;
            }
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

#[tauri::command(async)]
pub fn save_png(path: String, data: String) -> Res<()> {
    if !path.to_lowercase().ends_with(".png") {
        return Err("Сохранять можно только PNG".into());
    }
    std::fs::write(&path, png_bytes(&data)?).ctx("Не удалось сохранить файл")
}

#[tauri::command(async)]
pub fn copy_png<R: tauri::Runtime>(app: tauri::AppHandle<R>, data: String) -> Res<()> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let img = tauri::image::Image::from_bytes(&png_bytes(&data)?).ctx("Неверная картинка")?;
    app.clipboard().write_image(&img).ctx("Не удалось скопировать")
}

#[cfg(test)]
mod own_tests {
    use super::*;
    use crate::own::OwnMod;

    #[test]
    fn names_and_links_find_own_mods() {
        let mods = vec![
            OwnMod { id: "interactionpreview".into(), name: "Interaction Preview".into(), ..Default::default() },
            OwnMod { id: "plantsreimagined".into(), name: "PR: Plants Reimagined".into(), ..Default::default() },
        ];
        let id = |w: &str| own_match(&mods, w).map(|m| m.id.clone());
        assert_eq!(id("Interaction Preview").as_deref(), Some("interactionpreview"));
        assert_eq!(id("interaction-preview").as_deref(), Some("interactionpreview"));
        assert_eq!(id("Plants Reimagined").as_deref(), Some("plantsreimagined"));
        assert_eq!(id("pr-plants-reimagined").as_deref(), Some("plantsreimagined"));
        assert_eq!(id("Sodium"), None);
        assert_eq!(id("JEI"), None);
    }

    /// ANILAUNCHER_CATALOG_URL=http://127.0.0.1:8766/mods-catalog/ cargo test --lib friend_finds_own -- --ignored
    #[tokio::test]
    #[ignore]
    async fn friend_finds_own() {
        use tauri::Manager;
        let app = tauri::test::mock_app();
        app.manage(AppState::with_root(std::env::temp_dir().join("anilauncher-owntest2")));
        let text = "Мои моды:\nhttps://www.curseforge.com/minecraft/mc-mods/interaction-preview\n- Sodium\n- Interaction Preview".to_string();
        let r = resolve_share(app.state(), text, Some("fabric".into()), Some("1.21.11".into())).await.unwrap();
        let ids: Vec<_> = r.items.iter().map(|f| (f.project_id.clone(), f.available)).collect();
        println!("{ids:?} not found: {:?}", r.not_found);
        assert!(ids.contains(&("own:interactionpreview".to_string(), true)));
        assert!(r.items.iter().any(|f| f.slug == "sodium"));
        assert_eq!(r.items.iter().filter(|f| f.project_id == "own:interactionpreview").count(), 1, "no duplicate");
    }
}

#[cfg(test)]
mod pack_tests {
    use super::pack_kind;
    fn v(a: &[&str]) -> Vec<String> { a.iter().map(|s| s.to_string()).collect() }
    #[test]
    fn tells_resource_packs_from_shaders() {
        // resource pack with core shaders inside: still a resource pack
        assert_eq!(pack_kind(&v(&["pack.mcmeta", "pack.png", "assets/minecraft/shaders/core/rendertype_solid.fsh"])), Some("resourcepack"));
        assert_eq!(pack_kind(&v(&["Fresh/pack.mcmeta", "Fresh/assets/minecraft/shaders/post/x.json"])), Some("resourcepack"));
        // shader packs
        assert_eq!(pack_kind(&v(&["shaders/composite.fsh", "shaders/shaders.properties"])), Some("shader"));
        assert_eq!(pack_kind(&v(&["BSL_v8/shaders/final.fsh", "BSL_v8/shaders/lang/en_US.lang"])), Some("shader"));
        assert_eq!(pack_kind(&v(&["readme.txt"])), None);
    }
}

#[cfg(test)]
mod pack_live {
    /// cargo test --lib friend_finds_packs -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn friend_finds_packs() {
        use tauri::Manager;
        let app = tauri::test::mock_app();
        app.manage(crate::state::AppState::with_root(std::env::temp_dir().join("anilauncher-packtest")));
        let text = "Моды:\nSodium\nFresh Animations\nРесурспаки:\nFresh Animations\nFaithful 32x\nШейдеры:\nComplementary Reimagined\nBSL Shaders\n\nIris Shaders\nStay True".to_string();
        let r = super::resolve_share(app.state(), text, Some("fabric".into()), Some("1.21.11".into())).await.unwrap();
        for f in &r.items { println!("{:<28} {:<13} <- {}", f.title, f.project_type, f.source); }
        println!("not found: {:?}", r.not_found);
    }
}

#[cfg(test)]
mod list_tests {
    #[test]
    fn exported_lists_read_back() {
        assert_eq!(super::strip_version("Sodium 0.6.13+mc1.21.11"), "Sodium");
        assert_eq!(super::strip_version("Fabric API v0.141.6"), "Fabric API");
        assert_eq!(super::strip_version("Create 6"), "Create 6");
        assert_eq!(super::strip_version("1.21 Pack"), "1.21 Pack");
    }
}

#[cfg(test)]
mod list_live {
    /// cargo test --lib friend::list_live -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn exported_list_resolves() {
        use tauri::Manager;
        let app = tauri::test::mock_app();
        app.manage(crate::state::AppState::with_root(std::env::temp_dir().join("anilauncher-listlive")));
        let text = "Моя сборка — Minecraft 1.21.11, Fabric — 3 модов\n\nIris Shaders 1.10.4+mc1.21.11 — https://modrinth.com/project/YL57xq9U\nSodium 0.8.2+mc1.21.11 — https://modrinth.com/project/AANobbMI\nMod Menu 17.0.0".to_string();
        let r = super::resolve_share(app.state(), text, Some("fabric".into()), Some("1.21.11".into())).await.unwrap();
        let v = serde_json::to_value(&r).unwrap();
        println!("{}", serde_json::to_string_pretty(&v).unwrap().chars().take(1500).collect::<String>());
    }
}
