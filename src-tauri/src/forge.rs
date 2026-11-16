//! Forge and NeoForge: version lists and installation through their official installers.

use crate::net::{download_all, get_json, Download};
use crate::state::AppState;
use crate::util::{read_json, write_json, Ctx, Res};
use crate::versions::{ensure_java, Emit};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const FORGE_PROMOS: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEO_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const NEO_API: &str = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// NeoForge numbers its versions after Minecraft: 1.21.1 → 21.1.x, 26.3 → 26.3.0.x
fn neo_prefix(mc: &str) -> String {
    let parts: Vec<&str> = mc.split('.').collect();
    if parts.first() == Some(&"1") {
        format!("{}.{}.", parts.get(1).unwrap_or(&"0"), parts.get(2).unwrap_or(&"0"))
    } else {
        format!("{}.{}.{}.", parts.first().unwrap_or(&"0"), parts.get(1).unwrap_or(&"0"), parts.get(2).unwrap_or(&"0"))
    }
}

pub async fn versions_for(state: &AppState, loader: &str, mc: &str) -> Res<Vec<String>> {
    match loader {
        "forge" => {
            let xml = state
                .http
                .get(format!("{FORGE_MAVEN}/maven-metadata.xml"))
                .send()
                .await
                .ctx("Нет соединения с Forge")?
                .text()
                .await
                .ctx("Неверный ответ Forge")?;
            let prefix = format!("{mc}-");
            let mut list: Vec<String> = xml
                .split("<version>")
                .skip(1)
                .filter_map(|s| s.split("</version>").next())
                .filter(|v| v.starts_with(&prefix))
                .map(String::from)
                .collect();
            list.dedup();
            Ok(list)
        }
        "neoforge" => {
            let v: Value = get_json(&state.http, NEO_API).await?;
            let prefix = neo_prefix(mc);
            let mut list: Vec<String> = v
                .get("versions")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .filter(|x| x.starts_with(&prefix))
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            list.reverse();
            Ok(list)
        }
        _ => Ok(vec![]),
    }
}

pub async fn default_version(state: &AppState, loader: &str, mc: &str) -> Res<String> {
    if loader == "forge" {
        if let Ok(p) = get_json::<Value>(&state.http, FORGE_PROMOS).await {
            for key in [format!("{mc}-recommended"), format!("{mc}-latest")] {
                if let Some(v) = p.pointer(&format!("/promos/{key}")).and_then(|x| x.as_str()) {
                    return Ok(format!("{mc}-{v}"));
                }
            }
        }
    }
    let list = versions_for(state, loader, mc).await?;
    list.iter()
        .find(|v| !v.contains("beta"))
        .or_else(|| list.first())
        .cloned()
        .ok_or(format!("Для Minecraft {mc} нет {}", if loader == "forge" { "Forge" } else { "NeoForge" }))
}

#[tauri::command]
pub async fn list_forge_versions(state: State<'_, AppState>, loader: String, mc: String) -> Res<Vec<String>> {
    versions_for(&state, &loader, &mc).await
}

/// Finds the version profile an installer wrote for this loader version.
fn find_installed(state: &AppState, loader: &str, mc: &str, version: &str) -> Option<Value> {
    let short = version.strip_prefix(&format!("{mc}-")).unwrap_or(version);
    for e in std::fs::read_dir(state.versions()).ok()?.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let lower = id.to_lowercase();
        let matches = lower.contains(loader) && id.contains(short);
        if !matches {
            continue;
        }
        if let Some(v) = read_json::<Value>(&e.path().join(format!("{id}.json"))) {
            if v.get("inheritsFrom").and_then(|x| x.as_str()) == Some(mc) {
                return Some(v);
            }
        }
    }
    None
}

/// Runs the official installer once ("--installClient") into the shared launcher folder.
pub async fn ensure(
    state: &AppState,
    emit: &Emit,
    loader: &str,
    mc: &str,
    version: &str,
    vanilla: &Value,
) -> Res<Value> {
    if let Some(v) = find_installed(state, loader, mc, version) {
        if generated_files_present(state, mc, &v) {
            return Ok(v);
        }
        // The installer's output is gone or broken (antivirus, cleanup, an interrupted install):
        // clear its leftovers so it rebuilds them from scratch, then run it again.
        if let Some(dir) = generated_dir(state, mc, &v) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
    emit("loader", 0, 1);
    // The installer needs the vanilla client and a launcher profile file next to it.
    let client = vanilla.pointer("/downloads/client").ok_or("Нет клиента игры")?;
    let client_path = state.versions().join(mc).join(format!("{mc}.jar"));
    let installer_url = match loader {
        "forge" => format!("{FORGE_MAVEN}/{version}/forge-{version}-installer.jar"),
        _ => format!("{NEO_MAVEN}/{version}/neoforge-{version}-installer.jar"),
    };
    let installer = state
        .root
        .join("cache")
        .join("installers")
        .join(format!("{loader}-{version}-installer.jar"));
    download_all(
        &state.http,
        vec![
            Download {
                url: client.get("url").and_then(|u| u.as_str()).unwrap_or("").into(),
                path: client_path,
                sha1: client.get("sha1").and_then(|s| s.as_str()).map(String::from),
                size: client.get("size").and_then(|s| s.as_u64()),
            },
            Download { url: installer_url, path: installer.clone(), sha1: None, size: None },
        ],
        |_, _| {},
    )
    .await?;
    let profiles = state.root.join("launcher_profiles.json");
    if !profiles.exists() {
        write_json(&profiles, &serde_json::json!({"profiles": {}}))?;
    }
    let component = vanilla
        .pointer("/javaVersion/component")
        .and_then(|c| c.as_str())
        .unwrap_or("jre-legacy");
    let java = ensure_java(state, emit, component).await?;
    // javaw has no console; for the installer use java(.exe) next to it
    let java_cli: PathBuf = {
        let name = if cfg!(windows) { "java.exe" } else { "java" };
        java.with_file_name(name)
    };
    emit("loader", 0, 1);
    // Some Java builds compress the installer's jars differently and its checksums then fail;
    // if that happens, try again with any other Java the launcher (or the computer) has.
    let mut javas = vec![java_cli];
    for e in std::fs::read_dir(state.runtimes()).into_iter().flatten().flatten() {
        let bin = e.path().join("bin").join(if cfg!(windows) { "java.exe" } else { "java" });
        if bin.exists() && !javas.contains(&bin) {
            javas.push(bin);
        }
    }
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let bin = PathBuf::from(home).join("bin").join(if cfg!(windows) { "java.exe" } else { "java" });
        if bin.exists() && !javas.contains(&bin) {
            javas.push(bin);
        }
    }
    let log_path = state.root.join("cache").join("installers").join(format!("{loader}-{version}.log"));
    let mut log = String::new();
    let mut ok = false;
    for java_cli in javas {
        let root = state.root.clone();
        let installer = installer.clone();
        let output = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&java_cli);
            cmd.arg("-jar").arg(&installer).arg("--installClient").arg(&root).current_dir(&root);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            cmd.output()
        })
        .await
        .ctx("Установщик не запустился")?;
        let Ok(output) = output else { continue };
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        log.push_str(&text);
        let _ = std::fs::write(&log_path, &log);
        let failed = !output.status.success() || text.contains("There was an error during installation");
        if !failed {
            ok = true;
            break;
        }
        if let Some(dir) = find_installed(state, loader, mc, version).and_then(|v| generated_dir(state, mc, &v)) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
    match find_installed(state, loader, mc, version).filter(|v| ok && generated_files_present(state, mc, v)) {
        Some(v) => {
            remember_generated(state, mc, &v);
            Ok(v)
        }
        None => Err(format!(
            "Установщик {} не смог установить версию {version}. Лог: {}",
            if loader == "forge" { "Forge" } else { "NeoForge" },
            log_path.display()
        )),
    }
}

/// The installer produces the patched game jars itself; they are not in the library list, so
/// check for them by the MCP/NeoForm version named in the game arguments.
fn generated_dir(state: &AppState, mc: &str, v: &Value) -> Option<PathBuf> {
    let args: Vec<&str> = v
        .pointer("/arguments/game")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let mcp = args.windows(2).find(|w| w[0] == "--fml.mcpVersion" || w[0] == "--fml.neoFormVersion").map(|w| w[1].to_string())?;
    Some(state.libraries().join("net").join("minecraft").join("client").join(format!("{mc}-{mcp}")))
}

fn marker_path(state: &AppState, v: &Value) -> Option<PathBuf> {
    let id = v.get("id").and_then(|x| x.as_str())?;
    Some(state.versions().join(id).join("anilauncher-generated.json"))
}

/// After a good install: note which files the installer generated, to check them before each launch.
fn remember_generated(state: &AppState, mc: &str, v: &Value) {
    let (Some(dir), Some(marker)) = (generated_dir(state, mc, v), marker_path(state, v)) else { return };
    let files: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".jar"))
        .collect();
    if !files.is_empty() {
        let _ = write_json(&marker, &serde_json::json!({ "dir": dir, "files": files }));
    }
}

fn generated_files_present(state: &AppState, mc: &str, v: &Value) -> bool {
    // What we saw after our own install is the most reliable list.
    if let Some(m) = marker_path(state, v).and_then(|p| read_json::<Value>(&p)) {
        let dir = PathBuf::from(m.get("dir").and_then(|d| d.as_str()).unwrap_or(""));
        return m
            .get("files")
            .and_then(|f| f.as_array())
            .map(|f| f.iter().filter_map(|x| x.as_str()).all(|n| dir.join(n).exists()))
            .unwrap_or(true);
    }
    // Installed before this check existed: the Forge/NeoForge layout of these versions.
    let Some(dir) = generated_dir(state, mc, v) else { return true };
    if !dir.exists() {
        return true; // a layout we don't know about; trust the version file
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    ["srg", "extra"].iter().all(|k| dir.join(format!("client-{name}-{k}.jar")).exists())
}
