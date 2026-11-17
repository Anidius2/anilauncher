//! Default game settings: options every new build starts with (language, volumes, graphics, narrator…),
//! plus default resource packs and a shader pack installed into new builds.

use crate::instances;
use crate::state::AppState;
use crate::util::{Ctx, Res};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::State;

fn prefs(state: &AppState) -> Option<Value> {
    let d = state.settings().get("gameDefaults").cloned()?;
    if d.get("enabled").and_then(|v| v.as_bool()) == Some(false) {
        return None;
    }
    Some(d)
}

fn num(d: &Value, k: &str) -> Option<f64> {
    d.get(k).and_then(|v| v.as_f64())
}
fn flag(d: &Value, k: &str) -> Option<bool> {
    d.get(k).and_then(|v| v.as_bool())
}

/// options.txt lines for the chosen defaults (only what the player set; everything else stays the game's own).
pub fn option_lines(d: &Value) -> BTreeMap<String, String> {
    let mut o = BTreeMap::new();
    if let Some(l) = d.get("lang").and_then(|v| v.as_str()).filter(|l| !l.is_empty()) {
        o.insert("lang".into(), l.to_lowercase());
    }
    for (key, opt) in [("masterVolume", "soundCategory_master"), ("musicVolume", "soundCategory_music"), ("soundVolume", "soundCategory_block")] {
        if let Some(v) = num(d, key) {
            o.insert(opt.into(), format!("{:.2}", (v / 100.0).clamp(0.0, 1.0)));
        }
    }
    if let Some(g) = d.get("graphics").and_then(|v| v.as_str()) {
        let id = match g { "fast" => "0", "fabulous" => "2", _ => "1" };
        o.insert("graphicsMode".into(), id.into());
    }
    if let Some(v) = num(d, "renderDistance") {
        o.insert("renderDistance".into(), format!("{}", v.clamp(2.0, 32.0) as i64));
    }
    if let Some(v) = num(d, "fov") {
        // stored as (fov - 70) / 40
        o.insert("fov".into(), format!("{:.3}", ((v.clamp(30.0, 110.0)) - 70.0) / 40.0));
    }
    if let Some(v) = num(d, "guiScale") {
        o.insert("guiScale".into(), format!("{}", v.clamp(0.0, 6.0) as i64));
    }
    if let Some(v) = num(d, "maxFps") {
        o.insert("maxFps".into(), format!("{}", v.clamp(10.0, 260.0) as i64));
    }
    if let Some(v) = num(d, "brightness") {
        o.insert("gamma".into(), format!("{:.2}", (v / 100.0).clamp(0.0, 1.0)));
    }
    if let Some(b) = flag(d, "narrator") {
        o.insert("narrator".into(), if b { "1" } else { "0" }.into());
    }
    for (key, opt) in [("fullscreen", "fullscreen"), ("autoJump", "autoJump"), ("subtitles", "showSubtitles"), ("vsync", "enableVsync")] {
        if let Some(b) = flag(d, key) {
            o.insert(opt.into(), b.to_string());
        }
    }
    if flag(d, "skipOnboarding").unwrap_or(true) {
        o.insert("onboardAccessibility".into(), "false".into());
        o.insert("skipMultiplayerWarning".into(), "true".into());
    }
    o
}

/// The game's data version, read from the client jar on the classpath, so the game reads our options.txt
/// as current and doesn't try to "upgrade" it.
fn data_version(classpath: &[PathBuf]) -> Option<i64> {
    for jar in classpath.iter().filter(|p| p.to_string_lossy().contains("versions")) {
        let Ok(f) = std::fs::File::open(jar) else { continue };
        let Ok(mut z) = zip::ZipArchive::new(f) else { continue };
        let Ok(mut e) = z.by_name("version.json") else { continue };
        let mut s = String::new();
        if std::io::Read::read_to_string(&mut e, &mut s).is_ok() {
            if let Some(v) = serde_json::from_str::<Value>(&s).ok().and_then(|j| j.get("world_version").and_then(|v| v.as_i64())) {
                return Some(v);
            }
        }
    }
    None
}

fn read_options(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once(':').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect()
}

fn write_options(path: &Path, lines: &[(String, String)]) -> Res<()> {
    let text: String = lines.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    std::fs::write(path, text).ctx("Не удалось записать настройки игры")
}

fn merge(lines: &mut Vec<(String, String)>, set: &BTreeMap<String, String>) {
    for (k, v) in set {
        match lines.iter_mut().find(|(kk, _)| kk == k) {
            Some(l) => l.1 = v.clone(),
            None => lines.push((k.clone(), v.clone())),
        }
    }
}

fn pack_list(game_dir: &Path) -> Vec<String> {
    let mut packs: Vec<String> = std::fs::read_dir(game_dir.join("resourcepacks"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();
    packs.sort();
    packs
}

/// First launch of a build (no options.txt yet): write the defaults, switch on the packs that were
/// installed with the build and the default shader pack.
pub fn first_run(state: &AppState, instance_id: &str, game_dir: &Path, classpath: &[PathBuf]) {
    let Some(d) = prefs(state) else { return };
    let path = game_dir.join("options.txt");
    if path.exists() {
        return;
    }
    let mut lines: Vec<(String, String)> = Vec::new();
    if let Some(v) = data_version(classpath) {
        lines.push(("version".into(), v.to_string()));
    }
    merge(&mut lines, &option_lines(&d));
    // In options.txt the last pack is on top; in the launcher the first default pack is the most important.
    let mut packs = pack_list(game_dir);
    let wanted: Vec<String> = d
        .get("packs")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|p| p.get("projectId").and_then(|x| x.as_str()).map(String::from)).collect())
        .unwrap_or_default();
    let meta = crate::content::load_meta(state, instance_id);
    let rank = |file: &str| -> usize {
        meta.entries
            .iter()
            .find(|e| e.filename == file)
            .and_then(|e| e.project_id.as_ref())
            .and_then(|p| wanted.iter().position(|w| w == p))
            .unwrap_or(usize::MAX)
    };
    packs.sort_by_key(|f| std::cmp::Reverse(rank(f)));   // unknown packs first (bottom), the first default last (top)
    if !packs.is_empty() {
        let list: Vec<String> = std::iter::once("\"vanilla\"".to_string()).chain(packs.iter().map(|p| format!("\"file/{p}\""))).collect();
        lines.push(("resourcePacks".into(), format!("[{}]", list.join(","))));
    }
    let _ = write_options(&path, &lines);
    enable_shader(game_dir);
}

/// Turns on the (single) shader pack in Iris' settings, unless the player already chose something.
fn enable_shader(game_dir: &Path) {
    let iris = game_dir.join("config").join("iris.properties");
    if iris.exists() {
        return;
    }
    let shader = std::fs::read_dir(game_dir.join("shaderpacks"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| !n.starts_with('.') && !n.ends_with(".txt"));
    if let Some(s) = shader {
        let _ = std::fs::create_dir_all(game_dir.join("config"));
        // Iris and Oculus read config/iris.properties
        let _ = std::fs::write(iris, format!("enableShaders=true\nshaderPack={s}\n"));
        // OptiFine reads optionsshaders.txt
        let of = game_dir.join("optionsshaders.txt");
        if !of.exists() {
            let _ = std::fs::write(of, format!("shaderPack={s}\n"));
        }
    }
}

/// Installs the default resource packs and shader pack into a new build (and Iris for the shader,
/// where the loader can run it). Returns what was installed.
#[tauri::command]
pub async fn apply_new_build_defaults(state: State<'_, AppState>, instance_id: String) -> Res<Vec<String>> {
    let Some(d) = prefs(&state) else { return Ok(vec![]) };
    let inst = instances::load(&state, &instance_id)?;
    let mut done = Vec::new();
    let ids = |key: &str| -> Vec<String> {
        d.get(key)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|p| p.get("projectId").and_then(|x| x.as_str()).map(String::from)).collect())
            .unwrap_or_default()
    };
    for p in ids("packs") {
        if let Ok(t) = crate::content::install_content_inner(&state, &instance_id, "resourcepack", &p, None).await {
            done.extend(t);
        }
    }
    if let Some(sh) = d.get("shader").and_then(|s| s.get("projectId")).and_then(|x| x.as_str()) {
        // Shader packs need a shader mod. Fabric, Quilt, NeoForge: Iris (brings Sodium).
        // Forge up to 1.20.1: Oculus (the Iris port) with Embeddium. Newer Forge has no shader mod on Modrinth;
        // there the pack is still put in place and set in OptiFine's settings, for a player who adds OptiFine himself.
        let mc = inst.mc_version.as_str();
        let old_forge = inst.loader == "forge" && (mc == "1.20" || mc == "1.20.1" || mc.starts_with("1.19") || mc.starts_with("1.18") || mc.starts_with("1.16"));
        let shader_mod: Option<&[&str]> = match inst.loader.as_str() {
            "fabric" | "quilt" | "neoforge" => Some(&["YL57xq9U"]),               // Iris
            "forge" if old_forge => Some(&["sk9rgfiA", "GchcoXML"]),             // Embeddium, Oculus
            "forge" => Some(&[]),                                                // OptiFine by hand
            _ => None,
        };
        if let Some(mods) = shader_mod {
            for m in mods {
                let _ = crate::content::install_content_inner(&state, &instance_id, "mod", m, None).await;
            }
            if let Ok(t) = crate::content::install_content_inner(&state, &instance_id, "shader", sh, None).await {
                done.extend(t);
            }
        }
    }
    Ok(done)
}

/// Writes the default options into existing builds too (other settings of the build stay as they are).
#[tauri::command(async)]
pub fn apply_defaults_to_all(state: State<'_, AppState>) -> Res<usize> {
    let d = prefs(&state).ok_or("Базовые настройки выключены")?;
    let set = option_lines(&d);
    let running: Vec<String> = state.running.lock().map(|r| r.keys().cloned().collect()).unwrap_or_default();
    let mut n = 0;
    for inst in instances::list_instances(state.clone()) {
        if running.contains(&inst.id) {
            continue; // the game would overwrite the file on exit anyway
        }
        let path = instances::game_dir(&state, &inst.id).join("options.txt");
        if !path.exists() {
            continue; // gets the defaults on its first launch
        }
        let mut lines = read_options(&path);
        merge(&mut lines, &set);
        write_options(&path, &lines)?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_option_lines() {
        let d = serde_json::json!({"lang": "ru_RU", "musicVolume": 30, "graphics": "fabulous", "fov": 90, "narrator": false, "autoJump": false});
        let o = option_lines(&d);
        assert_eq!(o["lang"], "ru_ru");
        assert_eq!(o["soundCategory_music"], "0.30");
        assert_eq!(o["graphicsMode"], "2");
        assert_eq!(o["fov"], "0.500");
        assert_eq!(o["narrator"], "0");
        assert_eq!(o["autoJump"], "false");
        assert_eq!(o["onboardAccessibility"], "false");
        let mut lines = vec![("lang".to_string(), "en_us".to_string()), ("key_key.jump".to_string(), "key.keyboard.space".to_string())];
        merge(&mut lines, &o);
        assert_eq!(lines[0].1, "ru_ru");
        assert_eq!(lines[1].0, "key_key.jump");
    }
}

#[cfg(test)]
mod live {
    /// TEST_JAR=/path/to/1.21.11.jar cargo test --lib reads_data_version -- --ignored --nocapture
    #[test]
    #[ignore]
    fn reads_data_version() {
        let jar = std::path::PathBuf::from(std::env::var("TEST_JAR").unwrap());
        let v = super::data_version(&[jar]);
        println!("data version: {v:?}");
        assert!(v.unwrap_or(0) > 4000);
    }
}

#[cfg(test)]
mod live_build {
    /// TEST_JAR=... cargo test --lib new_build_gets_defaults -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn new_build_gets_defaults() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-defaults");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(crate::state::AppState::with_root(root.clone()));
        let st = app.state::<crate::state::AppState>();
        std::fs::create_dir_all(st.settings_path().parent().unwrap()).unwrap();
        std::fs::write(st.settings_path(), serde_json::json!({"gameDefaults": {
            "enabled": true, "lang": "ru_ru", "musicVolume": 20, "graphics": "fabulous", "narrator": false,
            "packs": [{"projectId": "50dA9Sha", "title": "Fresh Animations"}, {"projectId": "R6NEzAwj", "title": "Faithful 32x"}],
            "shader": {"projectId": "HVnmMxH1", "title": "Complementary Reimagined"}
        }}).to_string()).unwrap();
        let inst = crate::instances::Instance { id: "d1".into(), name: "t".into(),
            mc_version: std::env::var("TEST_MC").unwrap_or("1.21.11".into()), loader: std::env::var("TEST_LOADER").unwrap_or("fabric".into()), ..Default::default() };
        crate::instances::store(&st, &inst).unwrap();
        let got = super::apply_new_build_defaults(app.state(), "d1".into()).await.unwrap();
        println!("installed: {got:?}");
        let gd = crate::instances::game_dir(&st, "d1");
        super::first_run(&st, "d1", &gd, &[std::path::PathBuf::from(std::env::var("TEST_JAR").unwrap())]);
        println!("--- options.txt\n{}", std::fs::read_to_string(gd.join("options.txt")).unwrap());
        println!("--- iris.properties\n{}", std::fs::read_to_string(gd.join("config/iris.properties")).unwrap_or_default());
        for sub in ["mods", "resourcepacks", "shaderpacks"] {
            let files: Vec<_> = std::fs::read_dir(gd.join(sub)).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            println!("{sub}: {files:?}");
        }
    }
}
