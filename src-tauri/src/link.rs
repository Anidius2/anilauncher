//! AniLauncher Link: a small mod the launcher adds to Fabric/Quilt builds (from its own folder,
//! not from `mods/`), connected back to the launcher over 127.0.0.1 with a one-time token.
//! The game reports which world it is in and new screenshots; the launcher sends notifications,
//! turns resource packs on, and changes options and key bindings while the game runs.

use crate::instances;
use crate::state::AppState;
use crate::util::{read_json, write_json, Ctx, Res};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

type EmitFn = Arc<dyn Fn(Value) + Send + Sync>;

#[derive(Default)]
pub struct LinkHub {
    port: OnceLock<u16>,
    emit: OnceLock<EmitFn>,
    /// one-time token → instance id, created at launch
    pending: Mutex<HashMap<String, String>>,
    sessions: Mutex<HashMap<String, mpsc::UnboundedSender<String>>>,
    waiters: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    /// instance → current world message and when it was entered
    worlds: Mutex<HashMap<String, (Value, Instant)>>,
    next_id: AtomicU64,
}

/// Minecraft version → the companion jar built for it.
fn jar_for(mc: &str) -> Option<(&'static str, &'static [u8])> {
    let parts: Vec<u32> = mc.split('.').map(|p| p.parse().unwrap_or(u32::MAX)).collect();
    let (a, b, c) = (parts.first().copied()?, parts.get(1).copied().unwrap_or(0), parts.get(2).copied().unwrap_or(0));
    macro_rules! jar {
        ($v:literal) => {
            Some(($v, include_bytes!(concat!("../companion/anilink-", $v, ".jar")).as_slice()))
        };
    }
    match (a, b, c) {
        (1, 20, 0 | 1) => jar!("1.20.1"),
        (1, 20, 2) => jar!("1.20.2"),
        (1, 20, 3 | 4) => jar!("1.20.4"),
        (1, 20, 5 | 6) => jar!("1.20.6"),
        (1, 21, 0 | 1) => jar!("1.21.1"),
        (1, 21, 2 | 3) => jar!("1.21.3"),
        (1, 21, 4) => jar!("1.21.4"),
        (1, 21, 5) => jar!("1.21.5"),
        (1, 21, 6..=8) => jar!("1.21.8"),
        (1, 21, 9 | 10) => jar!("1.21.10"),
        (1, 21, 11) => jar!("1.21.11"),
        (26, 1, _) => jar!("26.1"),
        (26, 2, _) => jar!("26.2"),
        (26, 3, _) => jar!("26.3"),
        _ => None,
    }
}

/// How a Forge/NeoForge version can be given a mod from outside `mods/`.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Way {
    /// `--fml.mavenRoots` + `--fml.mods` (Forge ≤1.20.2, NeoForge ≤1.21.8)
    Maven,
    /// `-Dfml.modFolders=anilink%%<jar>` (NeoForge 1.21.9+)
    ModFolders,
    /// on the Java classpath, found by FML's classpath locator (Forge 1.20.3+)
    Classpath,
}

/// Forge/NeoForge: which of the two universal jars, and how to load it.
/// "srg": the game runs on obfuscated names (Forge 1.20–1.20.4, NeoForge 1.20.2–1.20.4);
/// "mojmap": Mojang names at runtime (Forge 1.20.6+, NeoForge 1.20.5+).
fn forge_plan(loader: &str, mc: &str, srg_runtime: Option<bool>) -> Option<(&'static str, &'static [u8], Way)> {
    let parts: Vec<u32> = mc.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    let v = (parts.first().copied()?, parts.get(1).copied().unwrap_or(0), parts.get(2).copied().unwrap_or(0));
    let srg = ("srg", include_bytes!("../companion/anilink-forge-srg.jar").as_slice());
    let moj = ("mojmap", include_bytes!("../companion/anilink-forge-mojmap.jar").as_slice());
    let (jar, way) = match loader {
        "forge" => match v {
            (1, 20, 0..=2) => (srg, Way::Maven),
            (1, 20, 3 | 4) => (srg, Way::Classpath),
            (1, 20, 5) => return None, // no Forge release
            (1, 20, _) | (1, 21, _) | (26, _, _) => (moj, Way::Classpath),
            _ => return None,
        },
        "neoforge" => match v {
            (1, 20, 2..=4) => (srg, Way::Maven),
            (1, 20, _) | (1, 21, 0..=8) => (moj, Way::Maven),
            (1, 21, _) | (26, _, _) => (moj, Way::ModFolders),
            _ => return None,
        },
        _ => return None,
    };
    // What the installed game actually uses beats the table (NeoForge 1.20.4 builds differ).
    let jar = match srg_runtime {
        Some(true) => srg,
        Some(false) => moj,
        None => jar,
    };
    Some((jar.0, jar.1, way))
}

/// Looks inside the patched game jar the loader runs: obfuscated member names (SRG) or Mojang names?
fn runtime_is_srg(state: &AppState, mc: &str, json: &Value) -> Option<bool> {
    let args: Vec<&str> = json.pointer("/arguments/game")?.as_array()?.iter().filter_map(|x| x.as_str()).collect();
    let mcp = args.windows(2).find(|w| w[0] == "--fml.mcpVersion" || w[0] == "--fml.neoFormVersion").map(|w| w[1])?;
    let dir = state.libraries().join("net").join("minecraft").join("client").join(format!("{mc}-{mcp}"));
    let jar = dir.join(format!("client-{mc}-{mcp}-srg.jar"));
    let file = std::fs::File::open(&jar).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut entry = zip.by_name("net/minecraft/client/Minecraft.class").ok()?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut bytes).ok()?;
    // Minecraft.getInstance() is m_91087_ in SRG
    Some(bytes.windows(8).any(|w| w == b"m_91087_"))
}

/// Arguments that load the companion: JVM options (before the main class) and game arguments (after it).
#[derive(Default, Debug)]
pub struct LinkArgs {
    pub jvm: Vec<String>,
    pub game: Vec<String>,
    /// extra entry for the Java classpath
    pub classpath: Option<PathBuf>,
}

impl LinkArgs {
    /// Adds everything to a full argument list (JVM options first, game arguments last).
    pub fn apply(self, args: &mut Vec<String>) {
        if let Some(extra) = &self.classpath {
            let sep = if cfg!(windows) { ";" } else { ":" };
            if let Some(i) = args.iter().position(|a| a == "-cp" || a == "-classpath") {
                if let Some(cp) = args.get_mut(i + 1) {
                    cp.push_str(sep);
                    cp.push_str(&extra.to_string_lossy());
                }
            }
        }
        for (i, a) in self.jvm.into_iter().enumerate() {
            args.insert(i, a);
        }
        args.extend(self.game);
    }
}

fn broken_path(state: &AppState) -> PathBuf {
    state.root.join("companion").join("disabled.json")
}

/// After a crash: did our mod cause it? Only its own code or its own loading errors count,
/// not merely being listed among the mods.
pub fn caused_crash(text: &str) -> bool {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"(?i)dev[./]anidius[./]anilink|AniLauncher Link has|AniLauncher Link[^\n]{0,80}requires|\(anilink\)[^\n]{0,120}requires").unwrap()
    })
    .is_match(text)
}

/// Switches the link off for this build on this exact version, so the next start goes without it.
pub fn mark_broken(state: &AppState, instance: &str, mc: &str, loader: &str) {
    let path = broken_path(state);
    let mut all: HashMap<String, Value> = read_json(&path).unwrap_or_default();
    all.insert(instance.to_string(), json!({"mc": mc, "loader": loader, "time": crate::util::now()}));
    let _ = write_json(&path, &all);
}

fn is_broken(state: &AppState, instance: &str, mc: &str, loader: &str) -> bool {
    let all: HashMap<String, Value> = read_json(&broken_path(state)).unwrap_or_default();
    all.get(instance).map(|v| v["mc"] == mc && v["loader"] == loader).unwrap_or(false)
}

pub fn enabled(state: &AppState) -> bool {
    state.settings().get("companion").and_then(|v| v.as_bool()).unwrap_or(true)
}

impl LinkHub {
    /// Starts listening once; later calls reuse the same port.
    pub fn ensure_started<R: Runtime>(self: &Arc<Self>, app: &AppHandle<R>, root: PathBuf) -> Res<u16> {
        if let Some(p) = self.port.get() {
            return Ok(*p);
        }
        let std_listener = std::net::TcpListener::bind(("127.0.0.1", 0)).ctx("Не удалось открыть связь с игрой")?;
        std_listener.set_nonblocking(true).ctx("Не удалось открыть связь с игрой")?;
        let port = std_listener.local_addr().ctx("Не удалось открыть связь с игрой")?.port();
        let app = app.clone();
        let _ = self.emit.set(Arc::new(move |v: Value| {
            let _ = app.emit("link", v);
        }));
        let _ = self.port.set(port);
        let hub = self.clone();
        tauri::async_runtime::spawn(async move {
            let Ok(listener) = tokio::net::TcpListener::from_std(std_listener) else { return };
            loop {
                let Ok((sock, addr)) = listener.accept().await else { continue };
                if !addr.ip().is_loopback() {
                    continue;
                }
                let hub = hub.clone();
                let root = root.clone();
                tauri::async_runtime::spawn(async move { hub.serve(sock, root).await });
            }
        });
        Ok(port)
    }

    pub fn new_token(&self, instance: &str) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        if let Ok(mut p) = self.pending.lock() {
            p.insert(token.clone(), instance.to_string());
        }
        token
    }

    fn emit(&self, v: Value) {
        if let Some(e) = self.emit.get() {
            e(v);
        }
    }

    async fn serve(self: Arc<Self>, sock: tokio::net::TcpStream, root: PathBuf) {
        let (r, mut w) = sock.into_split();
        let mut lines = BufReader::new(r).lines();
        // The first line must be the hello with a token we handed out.
        let Ok(Ok(Some(first))) = tokio::time::timeout(Duration::from_secs(10), lines.next_line()).await else { return };
        let Ok(hello) = serde_json::from_str::<Value>(&first) else { return };
        let token = hello.get("token").and_then(|v| v.as_str()).unwrap_or("");
        let Some(instance) = self.pending.lock().ok().and_then(|mut p| p.remove(token)) else { return };
        // keep the token valid for reconnects of the same game
        if let Ok(mut p) = self.pending.lock() {
            p.insert(token.to_string(), instance.clone());
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        if let Ok(mut s) = self.sessions.lock() {
            s.insert(instance.clone(), tx);
        }
        tauri::async_runtime::spawn(async move {
            while let Some(line) = rx.recv().await {
                if w.write_all(line.as_bytes()).await.is_err() || w.write_all(b"\n").await.is_err() {
                    break;
                }
            }
        });
        self.emit(json!({"instanceId": instance, "type": "connected"}));

        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            match msg.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                "world" => {
                    self.close_world(&root, &instance);
                    if msg.get("state").and_then(|v| v.as_str()) == Some("enter") {
                        if let Ok(mut w) = self.worlds.lock() {
                            w.insert(instance.clone(), (msg.clone(), Instant::now()));
                        }
                    }
                    self.emit(json!({"instanceId": instance, "type": "world", "world": msg}));
                }
                "screenshot" => {
                    let file = msg.get("file").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if !file.is_empty() && !file.contains(['/', '\\']) {
                        let path = instances::game_dir_at(&root, &instance).join("screenshots").join(&file);
                        save_shot_meta(&root, &path, &msg);
                        self.emit(json!({"instanceId": instance, "type": "screenshot", "path": path, "meta": msg}));
                    }
                }
                "reply" => {
                    if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                        if let Some(tx) = self.waiters.lock().ok().and_then(|mut w| w.remove(&id)) {
                            let _ = tx.send(msg);
                        }
                    }
                }
                _ => {}
            }
        }
        self.close_world(&root, &instance);
        if let Ok(mut s) = self.sessions.lock() {
            s.remove(&instance);
        }
        self.emit(json!({"instanceId": instance, "type": "disconnected"}));
    }

    /// Adds the time spent in the current world to the build's per-world totals.
    fn close_world(&self, root: &Path, instance: &str) {
        let Some((msg, since)) = self.worlds.lock().ok().and_then(|mut w| w.remove(instance)) else { return };
        let kind = msg.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() || kind == "other" {
            return;
        }
        let secs = since.elapsed().as_secs();
        if secs == 0 {
            return;
        }
        let path = world_time_path(root, instance);
        let mut all: HashMap<String, Value> = read_json(&path).unwrap_or_default();
        let key = format!("{}:{}", if kind == "singleplayer" { "sp" } else { "mp" }, id);
        let prev = all.get(&key).and_then(|v| v.get("seconds")).and_then(|v| v.as_u64()).unwrap_or(0);
        all.insert(
            key,
            json!({"seconds": prev + secs, "name": msg.get("name"), "kind": kind, "last": crate::util::now()}),
        );
        let _ = write_json(&path, &all);
    }

    pub async fn send(&self, instance: &str, mut cmd: Value) -> Res<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        cmd["id"] = json!(id);
        let tx = self
            .sessions
            .lock()
            .ok()
            .and_then(|s| s.get(instance).cloned())
            .ok_or("Игра не подключена к лаунчеру")?;
        let (otx, orx) = oneshot::channel();
        if let Ok(mut w) = self.waiters.lock() {
            w.insert(id, otx);
        }
        tx.send(cmd.to_string()).map_err(|_| "Игра не подключена к лаунчеру".to_string())?;
        let reply = tokio::time::timeout(Duration::from_secs(15), orx)
            .await
            .map_err(|_| "Игра не ответила".to_string())?
            .map_err(|_| "Игра не ответила".to_string())?;
        if reply.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            Ok(reply)
        } else {
            Err(reply.get("error").and_then(|v| v.as_str()).unwrap_or("Игра не смогла это сделать").to_string())
        }
    }

    pub fn status(&self) -> HashMap<String, Value> {
        let worlds = self.worlds.lock().map(|w| w.clone()).unwrap_or_default();
        self.sessions
            .lock()
            .map(|s| {
                s.keys()
                    .map(|k| (k.clone(), worlds.get(k).map(|(m, _)| m.clone()).unwrap_or(Value::Null)))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn world_time_path(root: &Path, instance: &str) -> PathBuf {
    root.join("instances").join(instance).join("launcher").join("world-time.json")
}

fn shot_meta_path(root: &Path) -> PathBuf {
    root.join("media").join("screenshot-meta.json")
}

fn save_shot_meta(root: &Path, path: &Path, msg: &Value) {
    let file = shot_meta_path(root);
    let mut all: HashMap<String, Value> = read_json(&file).unwrap_or_default();
    let mut meta = msg.clone();
    if let Some(o) = meta.as_object_mut() {
        o.remove("type");
        o.remove("file");
    }
    all.insert(path.to_string_lossy().to_string(), meta);
    // keep the file from growing forever
    if all.len() > 5000 {
        all.retain(|k, _| Path::new(k).exists());
    }
    let _ = write_json(&file, &all);
}

/// Puts the companion jar for this version into the launcher's own folder (never `mods/`) and
/// returns the arguments that load it. None when the version or loader isn't supported.
pub fn jvm_args<R: Runtime>(app: &AppHandle<R>, state: &AppState, instance: &str, mc: &str, loader: &str, version_json: &Value) -> Res<Option<LinkArgs>> {
    if !enabled(state) || is_broken(state, instance, mc, loader) {
        return Ok(None);
    }
    let dir = state.root.join("companion");
    let write = |path: &Path, bytes: &[u8]| -> Res<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ctx("Не удалось подготовить AniLauncher Link")?;
        }
        if std::fs::read(path).map(|b| b != bytes).unwrap_or(true) {
            std::fs::write(path, bytes).ctx("Не удалось подготовить AniLauncher Link")?;
        }
        Ok(())
    };
    let mut out = LinkArgs::default();
    match loader {
        "fabric" | "quilt" => {
            let Some((ver, bytes)) = jar_for(mc) else { return Ok(None) };
            let jar = dir.join(format!("anilink-{ver}.jar"));
            write(&jar, bytes)?;
            let jar_s = jar.to_string_lossy().to_string();
            out.jvm.push(format!("-Dfabric.addMods={jar_s}"));
            out.jvm.push(format!("-Dloader.addMods={jar_s}"));
        }
        "forge" | "neoforge" => {
            let Some((kind, bytes, way)) = forge_plan(loader, mc, runtime_is_srg(state, mc, version_json)) else { return Ok(None) };
            // laid out like a maven repository, which FML's maven locator needs and the others don't mind
            let maven = dir.join("maven");
            let artifact = format!("anilink-forge-{kind}");
            let jar = maven.join("dev").join("anidius").join(&artifact).join("1.0.0").join(format!("{artifact}-1.0.0.jar"));
            write(&jar, bytes)?;
            match way {
                Way::Maven => {
                    out.game.push("--fml.mavenRoots".into());
                    out.game.push(maven.to_string_lossy().to_string());
                    out.game.push("--fml.mods".into());
                    out.game.push(format!("dev.anidius:{artifact}:1.0.0"));
                }
                Way::ModFolders => out.jvm.push(format!("-Dfml.modFolders=anilink%%{}", jar.to_string_lossy())),
                Way::Classpath => out.classpath = Some(jar),
            }
        }
        _ => return Ok(None),
    }
    let port = state.link.ensure_started(app, state.root.clone())?;
    let token = state.link.new_token(instance);
    out.jvm.push(format!("-Danilink.port={port}"));
    out.jvm.push(format!("-Danilink.token={token}"));
    out.jvm.push(format!("-Danilink.instance={instance}"));
    Ok(Some(out))
}

// ---------- commands ----------

#[tauri::command]
pub async fn link_send(state: State<'_, AppState>, instance_id: String, cmd: Value) -> Res<Value> {
    let hub = state.link.clone();
    hub.send(&instance_id, cmd).await
}

#[tauri::command]
pub fn link_status(state: State<'_, AppState>) -> HashMap<String, Value> {
    state.link.status()
}

#[tauri::command]
pub fn screenshot_meta(state: State<'_, AppState>) -> HashMap<String, Value> {
    read_json(&shot_meta_path(&state.root)).unwrap_or_default()
}

#[tauri::command]
pub fn world_times(state: State<'_, AppState>, instance_id: String) -> HashMap<String, Value> {
    read_json(&world_time_path(&state.root, &instance_id)).unwrap_or_default()
}

/// Copies a resource pack file into a running build and turns it on in the game.
#[tauri::command]
pub async fn link_enable_pack(state: State<'_, AppState>, instance_id: String, file: String) -> Res<()> {
    let hub = state.link.clone();
    let name = Path::new(&file).file_name().map(|n| n.to_string_lossy().to_string()).ok_or("Нет файла")?;
    hub.send(&instance_id, json!({"cmd": "enablePack", "pack": format!("file/{name}")})).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::caused_crash;

    #[test]
    fn crash_blame() {
        assert!(caused_crash("java.lang.UnsupportedClassVersionError: dev/anidius/anilink/AniLinkForge has been compiled"));
        assert!(caused_crash("at dev.anidius.anilink.Link.run(Link.java:40)"));
        assert!(caused_crash("AniLauncher Link requires version [0.17.0, ∞) of fabricloader, which is missing!"));
        assert!(caused_crash("Mod 'AniLauncher Link' (anilink) 1.0.0 requires version 1.21.1 of 'Minecraft'"));
        // merely listed among the loaded mods: not our fault
        assert!(!caused_crash("\\t- anilink 1.0.0+mc1.21.11\\n\\tAniLauncher Link 1.0.0 (anilink)\\nCaused by: java.lang.NullPointerException at com.example.Other"));
    }
}
