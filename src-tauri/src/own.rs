//! The author's own mods, published from the launcher's repository (mods-catalog/) before they
//! are on Modrinth. They appear in the mod search next to Modrinth results and install the same way.

use crate::content::{folder, Entry, Meta};
use crate::instances;
use crate::state::AppState;
use crate::util::{now, read_json, write_json, Ctx, Res};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::sync::Mutex;

pub const PREFIX: &str = "own:";
/// Fabric API on Modrinth: most Fabric mods need it
pub const FABRIC_API: &str = "P7dR8mSH";

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Catalog {
    pub mods: Vec<OwnMod>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct OwnMod {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub icon: Option<String>,
    pub versions: Vec<OwnVersion>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct OwnVersion {
    pub version: String,
    pub mc: String,
    pub loaders: Vec<String>,
    pub file: String,
    pub sha1: String,
    pub size: u64,
    pub deps: Vec<String>,
}

/// Where the catalog lives: the `mods-catalog` folder of the repository the launcher is built from.
pub fn base_url() -> Option<String> {
    if let Some(u) = option_env!("ANILAUNCHER_CATALOG_URL").filter(|u| u.starts_with("https://") || u.starts_with("http://127.0.0.1")) {
        return Some(if u.ends_with('/') { u.to_string() } else { format!("{u}/") });
    }
    // https://github.com/OWNER/REPO/releases/latest/download/latest.json
    let upd = option_env!("ANILAUNCHER_UPDATE_URL")?;
    let rest = upd.strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    Some(format!("https://raw.githubusercontent.com/{owner}/{repo}/main/mods-catalog/"))
}

static CACHE: Mutex<Option<(u64, Catalog)>> = Mutex::new(None);

/// The catalog, fetched at most every 10 minutes; the last saved copy when offline.
pub async fn catalog(state: &AppState) -> Catalog {
    if let Some((at, c)) = CACHE.lock().ok().and_then(|g| g.clone()) {
        if now() < at + 600 {
            return c;
        }
    }
    let disk = state.root.join("cache").join("own-mods.json");
    let fresh = match base_url() {
        Some(base) => {
            let get = async { state.http.get(format!("{base}catalog.json")).send().await.ok()?.error_for_status().ok()?.json::<Catalog>().await.ok() };
            tokio::time::timeout(std::time::Duration::from_secs(8), get).await.ok().flatten()
        }
        None => None,
    };
    let c = match fresh {
        Some(c) => {
            let _ = write_json(&disk, &c);
            c
        }
        None => read_json(&disk).unwrap_or_default(),
    };
    if let Ok(mut g) = CACHE.lock() {
        *g = Some((now(), c.clone()));
    }
    c
}

fn fits(v: &OwnVersion, mc: &str, loader: &str) -> bool {
    v.mc == mc && v.loaders.iter().any(|l| l == loader)
}

/// The newest file of a mod for this game version and loader.
pub fn best<'a>(m: &'a OwnMod, mc: &str, loader: &str) -> Option<&'a OwnVersion> {
    m.versions.iter().filter(|v| fits(v, mc, loader)).max_by(|a, b| cmp_versions(&a.version, &b.version))
}

pub fn cmp_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let num = |s: &str| s.split(['.', '-', '+']).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    num(a).cmp(&num(b))
}

pub fn icon_url(m: &OwnMod) -> Option<String> {
    Some(format!("{}{}", base_url()?, m.icon.as_ref()?))
}

/// Mods matching a search that have a file for this build.
pub async fn search(state: &AppState, query: &str, mc: Option<&str>, loader: Option<&str>) -> Vec<OwnMod> {
    let q = query.trim().to_lowercase();
    catalog(state)
        .await
        .mods
        .into_iter()
        .filter(|m| q.is_empty() || m.name.to_lowercase().contains(&q) || m.id.contains(&q) || m.description.to_lowercase().contains(&q))
        .filter(|m| match (mc, loader) {
            (Some(mc), Some(l)) if !mc.is_empty() && !l.is_empty() => best(m, mc, l).is_some(),
            (Some(mc), _) if !mc.is_empty() => m.versions.iter().any(|v| v.mc == mc),
            _ => true,
        })
        .collect()
}

pub async fn find(state: &AppState, project: &str) -> Option<OwnMod> {
    let id = project.strip_prefix(PREFIX)?;
    catalog(state).await.mods.into_iter().find(|m| m.id == id)
}

/// Downloads a file of an own mod into the build and records it. Returns the entry and its dependencies.
pub async fn install(state: &AppState, instance_id: &str, project: &str, version: Option<&str>, meta: &mut Meta) -> Res<(Entry, Vec<String>)> {
    let inst = instances::load(state, instance_id)?;
    let m = find(state, project).await.ok_or("Этого мода больше нет в каталоге")?;
    let v = match version {
        Some(ver) => m.versions.iter().find(|v| v.version == ver && fits(v, &inst.mc_version, &inst.loader)),
        None => best(&m, &inst.mc_version, &inst.loader),
    }
    .ok_or_else(|| format!("Нет версии для Minecraft {} ({})", inst.mc_version, inst.loader))?
    .clone();
    if v.file.is_empty() || v.file.contains('/') || v.file.contains('\\') || v.file.contains("..") {
        return Err("Неверное имя файла".into());
    }
    let base = base_url().ok_or("Каталог модов не настроен")?;
    let bytes = state
        .http
        .get(format!("{base}{}", urlencode(&v.file)))
        .send()
        .await
        .ctx("Нет соединения")?
        .error_for_status()
        .map_err(|e| format!("Файл мода не найден ({e})"))?
        .bytes()
        .await
        .ctx("Не удалось скачать файл")?;
    if !v.sha1.is_empty() && hex::encode(Sha1::digest(&bytes)) != v.sha1 {
        return Err("Файл скачался с ошибкой, попробуй ещё раз".into());
    }
    let dir = instances::game_dir(state, instance_id).join(folder("mod")?);
    std::fs::create_dir_all(&dir).ctx("Не удалось создать папку")?;
    // an older file of the same mod goes away
    for old in meta.entries.iter().filter(|e| e.project_id.as_deref() == Some(project) && e.filename != v.file) {
        let _ = std::fs::remove_file(dir.join(&old.filename));
        let _ = std::fs::remove_file(dir.join(format!("{}.disabled", old.filename)));
    }
    std::fs::write(dir.join(&v.file), &bytes).ctx("Не удалось сохранить файл")?;
    let entry = Entry {
        filename: v.file.clone(),
        kind: "mod".into(),
        project_id: Some(project.to_string()),
        version_id: Some(v.version.clone()),
        version_number: Some(v.version.clone()),
        title: Some(m.name.clone()),
        icon: icon_url(&m),
        sha1: Some(v.sha1.clone()).filter(|s| !s.is_empty()),
        pinned: false,
    };
    meta.entries.retain(|e| !(e.kind == "mod" && (e.filename == entry.filename || e.project_id.as_deref() == Some(project))));
    meta.entries.push(entry.clone());
    Ok((entry, v.deps.clone()))
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ANILAUNCHER_CATALOG_URL=http://127.0.0.1:8766/mods-catalog/ cargo test --lib own_mod_installs -- --ignored
    #[tokio::test]
    #[ignore]
    async fn own_mod_installs() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-owntest");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(AppState::with_root(root.clone()));
        let st = app.state::<AppState>();
        let inst = instances::Instance { id: "own1".into(), name: "t".into(), mc_version: "1.21.11".into(), loader: "fabric".into(), ..Default::default() };
        instances::store(&st, &inst).unwrap();
        let found = search(&st, "preview", Some("1.21.11"), Some("fabric")).await;
        assert_eq!(found.len(), 1, "search finds the mod");
        assert!(search(&st, "", Some("1.20.1"), Some("fabric")).await.is_empty(), "no file for 1.20.1");
        let titles = crate::content::install_content_inner(&st, "own1", "mod", "own:interactionpreview", None).await.unwrap();
        println!("installed: {titles:?}");
        let mods = instances::game_dir(&st, "own1").join("mods");
        let files: Vec<_> = std::fs::read_dir(&mods).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        println!("files: {files:?}");
        assert!(files.iter().any(|f| f.starts_with("interactionpreview-1.0.1+mc1.21.11")));
        assert!(files.iter().any(|f| f.starts_with("fabric-api")), "Fabric API comes along");
    }
    #[test]
    fn picks_newest_for_build() {
        let m = OwnMod {
            id: "x".into(),
            versions: vec![
                OwnVersion { version: "1.0.1".into(), mc: "1.21.11".into(), loaders: vec!["fabric".into(), "quilt".into()], ..Default::default() },
                OwnVersion { version: "1.0.10".into(), mc: "1.21.11".into(), loaders: vec!["fabric".into()], ..Default::default() },
                OwnVersion { version: "2.0.0".into(), mc: "26.2".into(), loaders: vec!["fabric".into()], ..Default::default() },
            ],
            ..Default::default()
        };
        assert_eq!(best(&m, "1.21.11", "fabric").unwrap().version, "1.0.10");
        assert_eq!(best(&m, "1.21.11", "quilt").unwrap().version, "1.0.1");
        assert!(best(&m, "1.21.11", "forge").is_none());
        assert_eq!(urlencode("a+mc1.21.jar"), "a%2Bmc1.21.jar");
    }
}
