//! Game versions, mod loaders (Fabric, Quilt), Java runtimes and file installation.

use crate::net::{download_all, get_json, Download};
use crate::state::AppState;
use crate::util::{library_key, maven_path, mojang_os, read_json, write_json, Ctx, Res};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

/// Progress callback: (stage, done, total).
pub type Emit = Arc<dyn Fn(&str, usize, usize) + Send + Sync>;

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const JAVA_ALL: &str =
    "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const QUILT_META: &str = "https://meta.quiltmc.org/v3";
const LIBRARIES: &str = "https://libraries.minecraft.net/";
const RESOURCES: &str = "https://resources.download.minecraft.net/";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameVersion {
    id: String,
    kind: String,
}

#[tauri::command]
pub async fn list_game_versions(state: State<'_, AppState>, loader: String) -> Res<Vec<GameVersion>> {
    match loader.as_str() {
        "fabric" | "quilt" => {
            let base = if loader == "fabric" { FABRIC_META } else { QUILT_META };
            let list: Vec<Value> = get_json(&state.http, &format!("{base}/versions/game")).await?;
            Ok(list
                .iter()
                .filter_map(|v| {
                    Some(GameVersion {
                        id: v.get("version")?.as_str()?.to_string(),
                        kind: if v.get("stable").and_then(|s| s.as_bool()).unwrap_or(false) {
                            "release".into()
                        } else {
                            "snapshot".into()
                        },
                    })
                })
                .collect())
        }
        _ => {
            let manifest: Value = get_json(&state.http, MANIFEST).await?;
            Ok(manifest
                .get("versions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            let kind = v.get("type")?.as_str()?;
                            if kind != "release" && kind != "snapshot" {
                                return None;
                            }
                            Some(GameVersion {
                                id: v.get("id")?.as_str()?.to_string(),
                                kind: kind.to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default())
        }
    }
}

#[tauri::command]
pub async fn list_loader_versions(state: State<'_, AppState>, loader: String, mc: String) -> Res<Vec<String>> {
    let base = match loader.as_str() {
        "fabric" => FABRIC_META,
        "quilt" => QUILT_META,
        "forge" | "neoforge" => return crate::forge::versions_for(&state, &loader, &mc).await,
        _ => return Ok(vec![]),
    };
    let list: Vec<Value> = get_json(&state.http, &format!("{base}/versions/loader/{mc}")).await?;
    Ok(list
        .iter()
        .filter_map(|v| v.pointer("/loader/version")?.as_str().map(|s| s.to_string()))
        .collect())
}

/// Best default loader version: first stable Fabric, first non-beta Quilt.
pub async fn default_loader(state: &AppState, loader: &str, mc: &str) -> Res<String> {
    let base = if loader == "fabric" { FABRIC_META } else { QUILT_META };
    let list: Vec<Value> = get_json(&state.http, &format!("{base}/versions/loader/{mc}")).await?;
    let pick = list
        .iter()
        .find(|v| {
            let stable = v.pointer("/loader/stable").and_then(|s| s.as_bool());
            let version = v.pointer("/loader/version").and_then(|s| s.as_str()).unwrap_or("");
            stable.unwrap_or(!version.contains("beta"))
        })
        .or_else(|| list.first())
        .and_then(|v| v.pointer("/loader/version")?.as_str())
        .ok_or(format!("Для {mc} нет версии {loader}"))?;
    Ok(pick.to_string())
}

async fn vanilla_json(state: &AppState, mc: &str) -> Res<Value> {
    let path = state.versions().join(mc).join(format!("{mc}.json"));
    if let Some(v) = read_json::<Value>(&path) {
        return Ok(v);
    }
    let manifest: Value = get_json(&state.http, MANIFEST).await?;
    let url = manifest
        .get("versions")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().find(|v| v.get("id").and_then(|i| i.as_str()) == Some(mc)))
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .ok_or(format!("Версия {mc} не найдена"))?;
    let v: Value = get_json(&state.http, url).await?;
    write_json(&path, &v)?;
    Ok(v)
}

async fn loader_json(state: &AppState, loader: &str, mc: &str, version: &str) -> Res<Value> {
    let path = state
        .versions()
        .join(format!("{loader}-{version}-{mc}"))
        .join("profile.json");
    if let Some(v) = read_json::<Value>(&path) {
        return Ok(v);
    }
    let base = if loader == "fabric" { FABRIC_META } else { QUILT_META };
    let v: Value = get_json(&state.http, &format!("{base}/versions/loader/{mc}/{version}/profile/json")).await?;
    write_json(&path, &v)?;
    Ok(v)
}

/// Loader profile on top of the vanilla version: loader libraries first, its main class, extra arguments.
fn merge(mut base: Value, loader: &Value) -> Value {
    let mut seen = HashSet::new();
    let mut libs = Vec::new();
    for lib in loader
        .get("libraries")
        .and_then(|l| l.as_array())
        .into_iter()
        .flatten()
        .chain(base.get("libraries").and_then(|l| l.as_array()).into_iter().flatten())
    {
        let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let has_natives = lib.get("natives").is_some();
        let key = format!("{}{}", library_key(name), if has_natives { "#n" } else { "" });
        if name.is_empty() || seen.insert(key) {
            libs.push(lib.clone());
        }
    }
    base["libraries"] = Value::Array(libs);
    if let Some(main) = loader.get("mainClass") {
        base["mainClass"] = main.clone();
    }
    // Old Forge replaces the whole argument line
    if let Some(args) = loader.get("minecraftArguments") {
        base["minecraftArguments"] = args.clone();
    }
    for kind in ["game", "jvm"] {
        if let Some(extra) = loader.pointer(&format!("/arguments/{kind}")).and_then(|a| a.as_array()) {
            if base.get("arguments").is_some() {
                let target = base["arguments"][kind].as_array().cloned().unwrap_or_default();
                let mut all = target;
                all.extend(extra.iter().cloned());
                base["arguments"][kind] = Value::Array(all);
            } else if kind == "game" {
                let mut old = base
                    .get("minecraftArguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                for e in extra.iter().filter_map(|e| e.as_str()) {
                    old.push(' ');
                    old.push_str(e);
                }
                base["minecraftArguments"] = Value::String(old);
            } else {
                base["extraJvm"] = Value::Array(extra.clone());
            }
        }
    }
    base
}

/// Mojang rule list: is this entry for our system? Feature-gated entries are left out.
pub fn rules_allow(entry: &Value) -> bool {
    let Some(rules) = entry.get("rules").and_then(|r| r.as_array()) else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        let mut applies = true;
        if let Some(os) = rule.get("os") {
            if let Some(name) = os.get("name").and_then(|n| n.as_str()) {
                applies &= name == mojang_os();
            }
            if let Some(arch) = os.get("arch").and_then(|a| a.as_str()) {
                applies &= arch == "x86" && cfg!(target_pointer_width = "32");
            }
        }
        if rule.get("features").is_some() {
            applies = false;
        }
        if applies {
            allowed = rule.get("action").and_then(|a| a.as_str()) == Some("allow");
        }
    }
    allowed
}

pub struct Prepared {
    pub json: Value,
    pub mc: String,
    pub version_name: String,
    pub java: PathBuf,
    pub classpath: Vec<PathBuf>,
    pub natives_dir: PathBuf,
    pub assets_index: String,
    pub game_assets: PathBuf,
    pub logging_arg: Option<String>,
}

fn progress_fn(emit: &Emit, stage: &'static str) -> impl Fn(usize, usize) + Send + Sync + 'static {
    let emit = emit.clone();
    move |d, t| emit(stage, d, t)
}

/// Downloads everything an instance needs and returns what the launch step uses.
pub async fn prepare(
    state: &AppState,
    emit: &Emit,
    mc: &str,
    loader: &str,
    loader_version: Option<&str>,
    game_dir: &Path,
) -> Res<Prepared> {
    emit("meta", 0, 1);
    let vanilla = vanilla_json(state, mc).await?;
    let (json, version_name) = match loader {
        "fabric" | "quilt" => {
            let lv = match loader_version {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => default_loader(state, loader, mc).await?,
            };
            let profile = loader_json(state, loader, mc, &lv).await?;
            let name = profile
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(mc)
                .to_string();
            (merge(vanilla, &profile), name)
        }
        "forge" | "neoforge" => {
            let lv = match loader_version {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => crate::forge::default_version(state, loader, mc).await?,
            };
            let profile = crate::forge::ensure(state, emit, loader, mc, &lv, &vanilla).await?;
            let name = profile
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(mc)
                .to_string();
            (merge(vanilla, &profile), name)
        }
        _ => (vanilla, mc.to_string()),
    };

    // Libraries, client jar, natives
    let libs_root = state.libraries();
    let mut downloads = Vec::new();
    let mut classpath = Vec::new();
    let mut natives = Vec::new();
    for lib in json.get("libraries").and_then(|l| l.as_array()).into_iter().flatten() {
        if !rules_allow(lib) {
            continue;
        }
        let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if let Some(art) = lib.pointer("/downloads/artifact") {
            if let Some(rel) = art.get("path").and_then(|p| p.as_str()) {
                let path = libs_root.join(rel);
                if let Some(url) = art.get("url").and_then(|u| u.as_str()).filter(|u| !u.is_empty()) {
                    downloads.push(Download {
                        url: url.to_string(),
                        path: path.clone(),
                        sha1: art.get("sha1").and_then(|s| s.as_str()).map(String::from),
                        size: art.get("size").and_then(|s| s.as_u64()),
                    });
                }
                classpath.push(path);
            }
        } else if lib.get("downloads").is_none() && !name.is_empty() {
            if let Some(rel) = maven_path(name) {
                let base = lib
                    .get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or(LIBRARIES)
                    .trim_end_matches('/')
                    .to_string();
                let rel_url = rel.to_string_lossy().replace('\\', "/");
                let path = libs_root.join(&rel);
                downloads.push(Download {
                    url: format!("{base}/{rel_url}"),
                    path: path.clone(),
                    sha1: lib.get("sha1").and_then(|s| s.as_str()).map(String::from),
                    size: lib.get("size").and_then(|s| s.as_u64()),
                });
                classpath.push(path);
            }
        }
        if let Some(classifier) = lib
            .get("natives")
            .and_then(|n| n.get(mojang_os()))
            .and_then(|c| c.as_str())
        {
            let classifier = classifier.replace("${arch}", "64");
            if let Some(nat) = lib.pointer(&format!("/downloads/classifiers/{classifier}")) {
                if let (Some(rel), Some(url)) = (
                    nat.get("path").and_then(|p| p.as_str()),
                    nat.get("url").and_then(|u| u.as_str()),
                ) {
                    let path = libs_root.join(rel);
                    downloads.push(Download {
                        url: url.to_string(),
                        path: path.clone(),
                        sha1: nat.get("sha1").and_then(|s| s.as_str()).map(String::from),
                        size: nat.get("size").and_then(|s| s.as_u64()),
                    });
                    let exclude: Vec<String> = lib
                        .pointer("/extract/exclude")
                        .and_then(|e| e.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    natives.push((path, exclude));
                }
            }
        }
    }
    let client = json
        .pointer("/downloads/client")
        .ok_or("В описании версии нет клиента игры")?;
    let client_path = state.versions().join(mc).join(format!("{mc}.jar"));
    downloads.push(Download {
        url: client.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
        path: client_path.clone(),
        sha1: client.get("sha1").and_then(|s| s.as_str()).map(String::from),
        size: client.get("size").and_then(|s| s.as_u64()),
    });
    // Forge and NeoForge tell their bootstrap to skip "${version_name}.jar" on the classpath
    // (-DignoreList); the game jar must carry that name or Minecraft is loaded twice.
    let renamed_client = (version_name != mc && json.to_string().contains("ignoreList"))
        .then(|| state.versions().join(&version_name).join(format!("{version_name}.jar")));
    classpath.push(renamed_client.clone().unwrap_or_else(|| client_path.clone()));

    // Logging config
    let mut logging_arg = None;
    if let Some(log) = json.pointer("/logging/client") {
        if let (Some(file), Some(arg)) = (log.get("file"), log.get("argument").and_then(|a| a.as_str())) {
            let id = file.get("id").and_then(|i| i.as_str()).unwrap_or("client.xml");
            let path = state.assets().join("log_configs").join(id);
            downloads.push(Download {
                url: file.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                path: path.clone(),
                sha1: file.get("sha1").and_then(|s| s.as_str()).map(String::from),
                size: file.get("size").and_then(|s| s.as_u64()),
            });
            logging_arg = Some(arg.replace("${path}", &path.to_string_lossy()));
        }
    }
    downloads.retain(|d| !d.url.is_empty());
    download_all(&state.http, downloads, progress_fn(emit, "libraries")).await?;
    if let Some(dest) = &renamed_client {
        let same = std::fs::metadata(dest).ok().map(|m| m.len()) == std::fs::metadata(&client_path).ok().map(|m| m.len());
        if !same {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).ctx("Не удалось подготовить игру")?;
            }
            std::fs::copy(&client_path, dest).ctx("Не удалось подготовить игру")?;
        }
    }

    // Assets
    let index = json.get("assetIndex").ok_or("В описании версии нет ресурсов")?;
    let index_id = index.get("id").and_then(|i| i.as_str()).unwrap_or(mc).to_string();
    let index_path = state.assets().join("indexes").join(format!("{index_id}.json"));
    if read_json::<Value>(&index_path).is_none() {
        let url = index.get("url").and_then(|u| u.as_str()).ok_or("Нет ссылки на ресурсы")?;
        let v: Value = get_json(&state.http, url).await?;
        write_json(&index_path, &v)?;
    }
    let index_json: Value = read_json(&index_path).ok_or("Не удалось прочитать список ресурсов")?;
    let objects_root = state.assets().join("objects");
    let mut asset_dl = Vec::new();
    let mut names = Vec::new();
    if let Some(objects) = index_json.get("objects").and_then(|o| o.as_object()) {
        for (name, obj) in objects {
            let Some(hash) = obj.get("hash").and_then(|h| h.as_str()) else { continue };
            let prefix = &hash[..2];
            let path = objects_root.join(prefix).join(hash);
            asset_dl.push(Download {
                url: format!("{RESOURCES}{prefix}/{hash}"),
                path: path.clone(),
                sha1: Some(hash.to_string()),
                size: obj.get("size").and_then(|s| s.as_u64()),
            });
            names.push((name.clone(), path));
        }
    }
    download_all(&state.http, asset_dl, progress_fn(emit, "assets")).await?;
    let legacy_virtual = index_json.get("virtual").and_then(|v| v.as_bool()).unwrap_or(false);
    let map_resources = index_json.get("map_to_resources").and_then(|v| v.as_bool()).unwrap_or(false);
    let game_assets = if map_resources {
        game_dir.join("resources")
    } else if legacy_virtual {
        state.assets().join("virtual").join(&index_id)
    } else {
        state.assets()
    };
    if legacy_virtual || map_resources {
        for (name, src) in &names {
            let dest = game_assets.join(name);
            if !dest.exists() {
                if let Some(p) = dest.parent() {
                    let _ = std::fs::create_dir_all(p);
                }
                let _ = std::fs::copy(src, &dest);
            }
        }
    }

    // Natives (older versions ship them as archives to unpack)
    let natives_dir = game_dir.parent().unwrap_or(game_dir).join("natives");
    let _ = std::fs::remove_dir_all(&natives_dir);
    std::fs::create_dir_all(&natives_dir).ctx("Не удалось создать папку natives")?;
    for (jar, exclude) in &natives {
        extract_natives(jar, &natives_dir, exclude)?;
    }

    // Java
    let component = json
        .pointer("/javaVersion/component")
        .and_then(|c| c.as_str())
        .unwrap_or("jre-legacy")
        .to_string();
    let java = ensure_java(state, emit, &component).await?;

    Ok(Prepared {
        json,
        mc: mc.to_string(),
        version_name,
        java,
        classpath,
        natives_dir,
        assets_index: index_id,
        game_assets,
        logging_arg,
    })
}

fn extract_natives(jar: &Path, dest: &Path, exclude: &[String]) -> Res<()> {
    let file = std::fs::File::open(jar).ctx("Не удалось открыть natives")?;
    let mut zip = zip::ZipArchive::new(file).ctx("Повреждённый архив natives")?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).ctx("Повреждённый архив natives")?;
        let name = entry.name().to_string();
        if entry.is_dir() || exclude.iter().any(|e| name.starts_with(e.as_str())) || name.starts_with("META-INF") {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest.join(rel);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).ctx("Не удалось создать папку")?;
        }
        let mut f = std::fs::File::create(&out).ctx("Не удалось распаковать natives")?;
        std::io::copy(&mut entry, &mut f).ctx("Не удалось распаковать natives")?;
    }
    Ok(())
}

fn java_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "aarch64") => "windows-arm64",
        ("windows", "x86") => "windows-x86",
        ("windows", _) => "windows-x64",
        ("macos", "aarch64") => "mac-os-arm64",
        ("macos", _) => "mac-os",
        (_, "x86") => "linux-i386",
        _ => "linux",
    }
}

fn java_exe(dir: &Path) -> PathBuf {
    if cfg!(windows) {
        dir.join("bin").join("javaw.exe")
    } else if cfg!(target_os = "macos") {
        dir.join("jre.bundle/Contents/Home/bin/java")
    } else {
        dir.join("bin").join("java")
    }
}

/// Downloads the exact Java runtime Mojang uses for this version.
pub(crate) async fn ensure_java(state: &AppState, emit: &Emit, component: &str) -> Res<PathBuf> {
    let dir = state.runtimes().join(component);
    let exe = java_exe(&dir);
    let marker = dir.join(".anilauncher-ok");
    if marker.exists() && exe.exists() {
        return Ok(exe);
    }
    emit("java", 0, 1);
    let all: Value = get_json(&state.http, JAVA_ALL).await?;
    let manifest_url = all
        .pointer(&format!("/{}/{component}/0/manifest/url", java_platform()))
        .and_then(|u| u.as_str())
        .ok_or(format!("Mojang не даёт Java «{component}» для этой системы"))?
        .to_string();
    let manifest: Value = get_json(&state.http, &manifest_url).await?;
    let mut dl = Vec::new();
    let mut executables = Vec::new();
    if let Some(files) = manifest.get("files").and_then(|f| f.as_object()) {
        for (rel, f) in files {
            let path = dir.join(rel);
            match f.get("type").and_then(|t| t.as_str()) {
                Some("directory") => {
                    let _ = std::fs::create_dir_all(&path);
                }
                Some("file") => {
                    if let Some(raw) = f.pointer("/downloads/raw") {
                        dl.push(Download {
                            url: raw.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                            path: path.clone(),
                            sha1: raw.get("sha1").and_then(|s| s.as_str()).map(String::from),
                            size: raw.get("size").and_then(|s| s.as_u64()),
                        });
                        if f.get("executable").and_then(|e| e.as_bool()).unwrap_or(false) {
                            executables.push(path);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    download_all(&state.http, dl, progress_fn(emit, "java")).await?;
    #[cfg(unix)]
    for p in executables {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = executables;
    if !exe.exists() {
        return Err("Java скачалась не полностью".into());
    }
    std::fs::write(&marker, component).ctx("Не удалось сохранить Java")?;
    Ok(exe)
}
