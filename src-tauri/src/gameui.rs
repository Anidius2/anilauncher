//! The game's look from the launcher: before a launch, writes what the AniLauncher Link mod needs to
//! draw its menus (build name and art, the player, the launcher's colours) and a small resource pack
//! that repaints the game's buttons in the same colours.

use crate::auth::Account;
use crate::instances::{self, Instance};
use crate::state::AppState;
use crate::util::{Ctx, Res};
use image::{imageops, ImageFormat, Rgba, RgbaImage};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PACK_FILE: &str = "AniLauncher Theme.zip";

#[derive(Clone, Copy)]
struct Prefs {
    menu: bool,
    pause: bool,
    loading: bool,
    buttons: bool,
}

fn prefs(state: &AppState) -> (Prefs, Value) {
    let s = state.settings();
    let g = s.get("gameUi").cloned().unwrap_or(Value::Null);
    let flag = |k: &str| g.get(k).and_then(|v| v.as_bool()).unwrap_or(true);
    (
        Prefs { menu: flag("menu"), pause: flag("pause"), loading: flag("loading"), buttons: flag("buttons") },
        g.get("colors").cloned().unwrap_or(Value::Null),
    )
}

fn hex(colors: &Value, key: &str, fallback: &str) -> String {
    colors
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 7 && s.starts_with('#') && u32::from_str_radix(&s[1..], 16).is_ok())
        .unwrap_or_else(|| fallback.to_string())
}

fn rgb(h: &str) -> [u8; 3] {
    let v = u32::from_str_radix(&h[1..], 16).unwrap_or(0);
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

fn luminance(c: [u8; 3]) -> f32 {
    (0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32) / 255.0
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| (a[i] as f32 * (1.0 - t) + b[i] as f32 * t).round() as u8)
}

async fn download(state: &AppState, url: &str, to: &Path) -> Option<()> {
    if to.exists() {
        return Some(());
    }
    let bytes = state.http.get(url).send().await.ok()?.error_for_status().ok()?.bytes().await.ok()?;
    std::fs::write(to, &bytes).ok()
}

fn short_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// The picture behind the menus: the build's own art, otherwise one of the Mojang key arts the
/// launcher already keeps, picked by the build so it stays the same between launches.
async fn art_source(state: &AppState, inst: &Instance, dir: &Path) -> Option<PathBuf> {
    if let Some(a) = &inst.art {
        if a.kind == "file" {
            let p = PathBuf::from(&a.value);
            if p.is_file() {
                return Some(p);
            }
        } else if a.value.starts_with("http") {
            let p = dir.join(format!("art-{}", short_hash(&a.value)));
            if download(state, &a.value, &p).await.is_some() {
                return Some(p);
            }
        }
    }
    let mut heroes: Vec<PathBuf> = std::fs::read_dir(state.root.join("cache").join("hero"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    heroes.sort();
    if heroes.is_empty() {
        return None;
    }
    let pick = u64::from_str_radix(&short_hash(&inst.id)[..8], 16).unwrap_or(0) as usize % heroes.len();
    Some(heroes[pick].clone())
}

/// Converts any picture to a PNG no bigger than 1920px (the game reads only PNG).
fn to_png(src: &Path, out: &Path) -> Option<()> {
    let key = format!("{}:{}", src.display(), std::fs::metadata(src).ok()?.len());
    let key_file = out.with_extension("key");
    if out.exists() && std::fs::read_to_string(&key_file).ok().as_deref() == Some(key.as_str()) {
        return Some(());
    }
    let img = image::open(src).ok()?;
    let img = if img.width() > 1920 { img.resize(1920, 1920 * img.height() / img.width(), imageops::FilterType::Triangle) } else { img };
    img.to_rgba8().save_with_format(out, ImageFormat::Png).ok()?;
    std::fs::write(key_file, key).ok()
}

/// The face from a skin (with the hat layer), 64×64 with crisp pixels.
fn head_png(skin: &Path, out: &Path) -> Option<()> {
    let skin = image::open(skin).ok()?.to_rgba8();
    if skin.width() < 64 {
        return None;
    }
    let s = skin.width() / 64;
    let mut face = imageops::crop_imm(&skin, 8 * s, 8 * s, 8 * s, 8 * s).to_image();
    let hat = imageops::crop_imm(&skin, 40 * s, 8 * s, 8 * s, 8 * s).to_image();
    imageops::overlay(&mut face, &hat, 0, 0);
    imageops::resize(&face, 64, 64, imageops::FilterType::Nearest).save_with_format(out, ImageFormat::Png).ok()
}

fn newest_world(game_dir: &Path) -> Option<(String, String, u64)> {
    std::fs::read_dir(game_dir.join("saves"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|w| {
            let t = w.path().join("level.dat").metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            Some((w.file_name().to_string_lossy().to_string(), t))
        })
        .max_by_key(|x| x.1)
        .map(|(id, t)| (id.clone(), id, t))
}

fn loader_name(l: &str) -> &str {
    match l {
        "fabric" => "Fabric",
        "quilt" => "Quilt",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        _ => "Vanilla",
    }
}

/// Writes ui.json (and the pictures it points to). Returns its path for -Danilink.ui, or None when
/// every menu is switched off.
pub async fn prepare(state: &AppState, inst: &Instance, account: &Account) -> Option<PathBuf> {
    let (p, colors) = prefs(state);
    if !(p.menu || p.pause || p.loading) {
        return None;
    }
    let dir = state.root.join("companion").join("ui").join(&inst.id);
    std::fs::create_dir_all(&dir).ok()?;
    let art = match art_source(state, inst, &dir).await {
        Some(src) => to_png(&src, &dir.join("art.png")).map(|_| dir.join("art.png")),
        None => None,
    };
    let head = match &account.skin_url {
        Some(url) => {
            let skin = dir.join(format!("skin-{}.png", short_hash(url)));
            match download(state, url, &skin).await.and_then(|_| head_png(&skin, &dir.join("head.png"))) {
                Some(()) => Some(dir.join("head.png")),
                None => None,
            }
        }
        None => None,
    };
    let game_dir = instances::game_dir(state, &inst.id);
    let world = newest_world(&game_dir).map(|(id, name, t)| json!({"id": id, "name": name, "time": t}));
    let server = inst.recent_servers.first().map(|s| json!({"address": s.address, "time": s.time}));
    let ui = json!({
        "menu": p.menu,
        "pause": p.pause,
        "loading": p.loading,
        "name": inst.name,
        "subtitle": format!("Minecraft {} · {}", inst.mc_version, loader_name(&inst.loader)),
        "nick": account.name,
        "licensed": account.kind == "msa",
        "art": art.map(|a| a.to_string_lossy().to_string()),
        "head": head.map(|h| h.to_string_lossy().to_string()),
        "colors": {
            "bg": hex(&colors, "bg", "#1b1b1b"),
            "panel": hex(&colors, "panel", "#2e2e2e"),
            "text": hex(&colors, "text", "#f2f2f2"),
            "muted": hex(&colors, "muted", "#b8b8b8"),
            "accent": hex(&colors, "accent", "#3c8527"),
        },
        "world": world,
        "server": server,
    });
    let path = dir.join("ui.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&ui).ok()?).ok()?;
    Some(path)
}

// ---------- the button pack ----------

/// A rounded flat button in the launcher's style: fill, a light top edge, a darker bottom lip.
fn button(w: u32, h: u32, fill: [u8; 3], edge: [u8; 3], lip: [u8; 3]) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let corner = (x == 0 || x == w - 1) && (y == 0 || y == h - 1);
            if corner {
                continue;
            }
            let border = x == 0 || x == w - 1 || y == 0 || y == h - 1;
            let c = if border {
                edge
            } else if y >= h - 3 {
                lip
            } else if y == 1 {
                mix(fill, [255, 255, 255], 0.08)
            } else {
                fill
            };
            img.put_pixel(x, y, Rgba([c[0], c[1], c[2], 255]));
        }
    }
    img
}

fn png_bytes(img: &RgbaImage) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let _ = img.write_to(&mut out, ImageFormat::Png);
    out.into_inner()
}

/// Builds the pack for the current colours. Same colours give the same bytes, so nothing is rewritten.
fn pack_bytes(colors: &Value) -> Res<Vec<u8>> {
    let panel = rgb(&hex(colors, "panel", "#2e2e2e"));
    let accent = rgb(&hex(colors, "accent", "#3c8527"));
    // the game always writes white text on buttons: keep them dark even with a light launcher theme
    let base = if luminance(panel) > 0.35 { mix(panel, [0, 0, 0], 0.72) } else { panel };
    let edge = mix(base, [255, 255, 255], 0.16);
    let lip = mix(base, [0, 0, 0], 0.35);
    let hi_fill = mix(base, accent, 0.35);
    let accent_dark = if luminance(accent) > 0.6 { mix(accent, [0, 0, 0], 0.4) } else { accent };

    let nine = |w: u32, h: u32, border: u32| json!({"gui": {"scaling": {"type": "nine_slice", "width": w, "height": h, "border": border}}}).to_string();
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("pack.mcmeta".into(), json!({"pack": {
            "description": "AniLauncher: buttons in the launcher's colours",
            "pack_format": 34, "supported_formats": [34, 999], "min_format": 34, "max_format": 999
        }}).to_string().into_bytes()),
    ];
    let sprites = "assets/minecraft/textures/gui/sprites/widget/";
    let mut add = |name: &str, img: RgbaImage, w: u32, h: u32, border: u32| {
        files.push((format!("{sprites}{name}.png"), png_bytes(&img)));
        files.push((format!("{sprites}{name}.png.mcmeta"), nine(w, h, border).into_bytes()));
    };
    add("button", button(200, 20, base, edge, lip), 200, 20, 3);
    add("button_highlighted", button(200, 20, hi_fill, accent_dark, mix(hi_fill, [0, 0, 0], 0.35)), 200, 20, 3);
    add("button_disabled", button(200, 20, mix(base, [0, 0, 0], 0.35), mix(base, [0, 0, 0], 0.1), mix(base, [0, 0, 0], 0.5)), 200, 20, 3);
    add("slider", button(200, 20, mix(base, [0, 0, 0], 0.25), edge, mix(base, [0, 0, 0], 0.45)), 200, 20, 3);
    add("slider_highlighted", button(200, 20, mix(base, [0, 0, 0], 0.25), accent_dark, mix(base, [0, 0, 0], 0.45)), 200, 20, 3);
    add("slider_handle", button(8, 20, edge, mix(edge, [255, 255, 255], 0.2), lip), 8, 20, 3);
    add("slider_handle_highlighted", button(8, 20, accent_dark, mix(accent_dark, [255, 255, 255], 0.3), mix(accent_dark, [0, 0, 0], 0.35)), 8, 20, 3);
    add("text_field", button(200, 20, mix(base, [0, 0, 0], 0.45), edge, mix(base, [0, 0, 0], 0.45)), 200, 20, 2);
    add("text_field_highlighted", button(200, 20, mix(base, [0, 0, 0], 0.45), accent_dark, mix(base, [0, 0, 0], 0.45)), 200, 20, 2);

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    // fixed timestamps: identical input -> identical zip
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (name, bytes) in files {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Keeps the button pack in the build's resourcepacks and switches it on once. If the player turns
/// it off in the game, it stays off. Only for 1.21+, where the buttons are separate pictures.
pub fn sync_pack(state: &AppState, inst: &Instance, classpath: &[PathBuf]) -> Res<()> {
    let (p, colors) = prefs(state);
    let game_dir = instances::game_dir(state, &inst.id);
    let packs = game_dir.join("resourcepacks");
    let file = packs.join(PACK_FILE);
    let marker = state.root.join("companion").join("ui").join(&inst.id).join("pack-enabled");
    let modern = inst.mc_version.split('.').next().map(|m| m != "1").unwrap_or(false)
        || inst.mc_version.split('.').nth(1).and_then(|m| m.parse::<u32>().ok()).map(|m| m >= 21).unwrap_or(false);
    if !p.buttons || !modern {
        if file.exists() {
            let _ = std::fs::remove_file(&file);
        }
        let _ = std::fs::remove_file(&marker);
        return Ok(());
    }
    std::fs::create_dir_all(&packs).ctx("Не удалось подготовить ресурс-пак")?;
    let bytes = pack_bytes(&colors)?;
    if std::fs::read(&file).map(|b| b != bytes).unwrap_or(true) {
        std::fs::write(&file, &bytes).ctx("Не удалось записать ресурс-пак")?;
    }
    if marker.exists() {
        return Ok(());
    }
    let entry = format!("file/{PACK_FILE}");
    let options = game_dir.join("options.txt");
    let mut lines = crate::defaults::read_options(&options);
    if lines.is_empty() {
        if let Some(v) = crate::defaults::data_version(classpath) {
            lines.push(("version".into(), v.to_string()));
        }
        // a fresh build: skip the one-time accessibility prompt, the launcher has its own settings
        lines.push(("onboardAccessibility".into(), "false".into()));
    }
    let mut list: Vec<String> = lines
        .iter()
        .find(|(k, _)| k == "resourcePacks")
        .and_then(|(_, v)| serde_json::from_str(v).ok())
        .unwrap_or_else(|| vec!["vanilla".to_string()]);
    if !list.contains(&entry) {
        // right above the game's own packs: any other pack the player chose stays on top of ours
        let at = list.iter().rposition(|x| x == "vanilla" || x == "fabric").map(|i| i + 1).unwrap_or(0);
        list.insert(at, entry);
    }
    let value = serde_json::to_string(&list).unwrap_or_default();
    match lines.iter_mut().find(|(k, _)| k == "resourcePacks") {
        Some(l) => l.1 = value,
        None => lines.push(("resourcePacks".into(), value)),
    }
    crate::defaults::write_options(&options, &lines)?;
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&marker, "1");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_is_stable_and_complete() {
        let colors = json!({"panel": "#2e2e2e", "accent": "#3c8527"});
        let a = pack_bytes(&colors).unwrap();
        let b = pack_bytes(&colors).unwrap();
        assert_eq!(a, b, "same colours must give the same file");
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(a)).unwrap();
        let names: Vec<String> = (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect();
        assert!(names.contains(&"pack.mcmeta".to_string()));
        assert!(names.contains(&"assets/minecraft/textures/gui/sprites/widget/button.png".to_string()));
        assert!(names.contains(&"assets/minecraft/textures/gui/sprites/widget/button.png.mcmeta".to_string()));
        assert_ne!(pack_bytes(&json!({"panel": "#ffffff", "accent": "#7c3aed"})).unwrap(), b);
    }

    #[test]
    fn light_theme_keeps_buttons_dark() {
        let bytes = pack_bytes(&json!({"panel": "#ffffff", "accent": "#3c7a24"})).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut f = z.by_name("assets/minecraft/textures/gui/sprites/widget/button.png").unwrap();
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut buf).unwrap();
        let img = image::load_from_memory(&buf).unwrap().to_rgba8();
        let c = img.get_pixel(100, 10);
        assert!(luminance([c[0], c[1], c[2]]) < 0.4, "white text must stay readable");
    }
}

#[cfg(test)]
mod live {
    /// Real game under Xvfb: DISPLAY=:99 TEST_MC=1.21.11 cargo test --lib gameui::live -- --ignored --nocapture
    /// Screenshots go to /tmp/ui-<mc>-*.png
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn menus_in_a_real_game() {
        use crate::{auth, instances, launch, state, util};
        use tauri::Manager;
        let mc = std::env::var("TEST_MC").unwrap_or_else(|_| "1.21.11".into());
        let wait: u64 = std::env::var("TEST_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(80);
        let root = std::env::temp_dir().join("anilauncher-test");
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let mut settings = st.settings();
        settings["lang"] = "ru".into();
        settings["companion"] = true.into();
        settings["gameUi"] = serde_json::json!({"menu": true, "pause": true, "loading": true, "buttons": true,
            "colors": {"bg": "#1b1b1b", "panel": "#2e2e2e", "text": "#f2f2f2", "muted": "#b8b8b8", "accent": "#3c8527"}});
        std::fs::write(st.settings_path(), serde_json::to_vec(&settings).unwrap()).unwrap();
        let file = auth::AccountsFile {
            accounts: vec![auth::Account { id: "m".into(), name: "Lic".into(), kind: "msa".into(), ..Default::default() }, auth::Account { id: "o".into(), name: "Anidius".into(), kind: "offline".into(), uuid: util::offline_uuid("Anidius"), ..Default::default() }],
            active: Some("o".into()),
        };
        auth::save(&st, &file).unwrap();
        let id = format!("ui-{}{}", std::env::var("TEST_LOADER").map(|l| format!("{l}-")).unwrap_or_default(), mc.replace('.', "_"));
        let inst = instances::Instance {
            id: id.clone(), name: "Выживание с друзьями".into(), mc_version: mc.clone(),
            loader: std::env::var("TEST_LOADER").unwrap_or_else(|_| "fabric".into()), loader_version: std::env::var("TEST_LOADER_VERSION").ok(),
            memory_mb: Some(2048), width: Some(1280), height: Some(720), jvm_args: std::env::var("TEST_JVM").ok(), ..Default::default()
        };
        let game = instances::game_dir(&st, &id);
        std::fs::create_dir_all(game.join("saves")).unwrap();
        instances::store(&st, &inst).unwrap();
        let world_src = std::path::Path::new("/tmp/anilauncher-test/instances/ui-1_21_11/minecraft/saves/Test World");
        let world = game.join("saves").join("Test World");
        if world_src.exists() && !world.exists() {
            let _ = std::process::Command::new("cp").arg("-r").arg(world_src).arg(&world).status();
        }
        let shot = |name: &str| {
            let out = format!("/tmp/ui-{mc}-{name}.png");
            let _ = std::process::Command::new("import").args(["-window", "root", &out]).status();
            println!("shot {out}");
        };
        let run = |quick: Option<launch::QuickPlay>| {
            let app = app.handle().clone();
            let id = id.clone();
            async move { launch::launch_instance(app.clone(), app.state(), id, quick).await.unwrap() }
        };
        // 1) loading screen and main menu
        run(None).await;
        let step: u64 = std::env::var("TEST_STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
        for i in 0..(50 / step) {
            tokio::time::sleep(std::time::Duration::from_secs(step)).await;
            shot(&format!("loading{i}"));
        }
        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        shot("menu");
        if std::env::var("TEST_CLICKS").is_ok() {
            let click = |x: &str, y: &str| {
                let _ = std::process::Command::new("xdotool").args(["mousemove", x, y, "click", "1"]).status();
            };
            click("360", "257"); // Continue
            tokio::time::sleep(std::time::Duration::from_secs(wait * 2)).await;
            shot("continued");
            let _ = std::process::Command::new("xdotool").args(["key", "Escape"]).status();
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            shot("pause");
            click("300", "635"); // Save and Quit
            tokio::time::sleep(std::time::Duration::from_secs(25)).await;
            shot("after-quit");
        }
        let _ = launch::stop_instance(app.state(), id.clone());
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        // 2) straight into the world, then Esc
        if world.exists() && std::env::var("TEST_NO_WORLD").is_err() && std::env::var("TEST_CLICKS").is_err() {
            run(Some(launch::QuickPlay { kind: "world".into(), target: "Test World".into() })).await;
            tokio::time::sleep(std::time::Duration::from_secs(wait * 3)).await;
            let _ = std::process::Command::new("xdotool").args(["key", "Escape"]).status();
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            shot("pause");
            let _ = launch::stop_instance(app.state(), id.clone());
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        let log = std::fs::read_to_string(instances::dir(&st, &id).join("logs/latest-launch.log")).unwrap_or_default();
        for l in log.lines().filter(|l| l.contains("AniLink") || l.contains("anilink") || l.contains("Mixin") && l.contains("ERROR")) {
            println!("LOG {l}");
        }
        println!("options resourcePacks: {:?}", std::fs::read_to_string(game.join("options.txt")).unwrap_or_default().lines().find(|l| l.starts_with("resourcePacks")));
    }
}
