//! Building the Java command line and starting the game.

use crate::auth;
use crate::crash;
use crate::instances;
use crate::state::AppState;
use crate::util::{classpath_sep, now, Ctx, Res};
use crate::versions::{prepare, rules_allow, Emit, Prepared};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn collect_args(list: Option<&Value>, out: &mut Vec<String>) {
    for item in list.and_then(|l| l.as_array()).into_iter().flatten() {
        match item {
            Value::String(s) => out.push(s.clone()),
            Value::Object(_) => {
                if !rules_allow(item) {
                    continue;
                }
                match item.get("value") {
                    Some(Value::String(s)) => out.push(s.clone()),
                    Some(Value::Array(a)) => out.extend(a.iter().filter_map(|x| x.as_str().map(String::from))),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn substitute(arg: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = arg.to_string();
    for (k, v) in vars {
        let key = format!("${{{k}}}");
        if out.contains(&key) {
            out = out.replace(&key, v);
        }
    }
    out
}

/// Full Java command line (after the java executable) for a prepared build.
/// Where to go right after the game opens: a world (1.20+) or a server.
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuickPlay {
    /// "world" or "server"
    pub kind: String,
    pub target: String,
}

pub fn build_args(
    state: &AppState,
    inst: &instances::Instance,
    account: &auth::Account,
    p: &Prepared,
    game_dir: &Path,
    quick: Option<&QuickPlay>,
) -> Res<Vec<String>> {
    let cp = p
        .classpath
        .iter()
        .map(|x| x.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(classpath_sep());
    let is_msa = account.kind == "msa";
    let token = if is_msa {
        account.mc_token.clone().unwrap_or_default()
    } else {
        "0".to_string()
    };
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", account.name.clone());
    vars.insert("version_name", p.version_name.clone());
    vars.insert("game_directory", game_dir.to_string_lossy().into());
    vars.insert("assets_root", state.assets().to_string_lossy().into());
    vars.insert("game_assets", p.game_assets.to_string_lossy().into());
    vars.insert("assets_index_name", p.assets_index.clone());
    vars.insert("auth_uuid", account.uuid.clone());
    vars.insert("auth_access_token", token.clone());
    vars.insert("auth_session", format!("token:{token}:{}", account.uuid));
    vars.insert("clientid", state.client_id().unwrap_or_default());
    vars.insert("auth_xuid", account.xuid.clone().unwrap_or_default());
    vars.insert("user_type", if is_msa { "msa" } else { "legacy" }.into());
    vars.insert(
        "version_type",
        p.json.get("type").and_then(|t| t.as_str()).unwrap_or("release").into(),
    );
    vars.insert("natives_directory", p.natives_dir.to_string_lossy().into());
    vars.insert("library_directory", state.libraries().to_string_lossy().into());
    vars.insert("classpath_separator", classpath_sep().into());
    vars.insert("launcher_name", "AniLauncher".into());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    vars.insert("classpath", cp.clone());
    vars.insert("user_properties", "{}".into());

    let memory = inst.memory_mb.unwrap_or_else(|| state.memory_mb()).clamp(1024, 65536);
    let mut args: Vec<String> = vec![format!("-Xmx{memory}M"), "-Xms512M".into()];
    // launcher-wide arguments first, so a build's own ones can override them
    if let Some(global) = state.settings().get("globalJvmArgs").and_then(|v| v.as_str()) {
        args.extend(global.split_whitespace().filter(|a| a.starts_with('-')).map(String::from));
    }
    if let Some(extra) = &inst.jvm_args {
        args.extend(extra.split_whitespace().map(String::from));
    }
    if let Some(log) = &p.logging_arg {
        args.push(log.clone());
    }
    let mut jvm = Vec::new();
    if p.json.pointer("/arguments/jvm").is_some() {
        collect_args(p.json.pointer("/arguments/jvm"), &mut jvm);
    } else {
        jvm.push("-Djava.library.path=${natives_directory}".into());
        jvm.push("-cp".into());
        jvm.push("${classpath}".into());
    }
    collect_args(p.json.get("extraJvm"), &mut jvm);
    args.extend(jvm.iter().map(|a| substitute(a, &vars)));
    let main_class = p
        .json
        .get("mainClass")
        .and_then(|m| m.as_str())
        .ok_or("Не найден главный класс игры")?;
    args.push(main_class.to_string());
    let mut game = Vec::new();
    if p.json.pointer("/arguments/game").is_some() {
        collect_args(p.json.pointer("/arguments/game"), &mut game);
    } else if let Some(old) = p.json.get("minecraftArguments").and_then(|a| a.as_str()) {
        game.extend(old.split_whitespace().map(String::from));
    }
    args.extend(game.iter().map(|a| substitute(a, &vars)));
    // Window size and full screen
    if inst.fullscreen {
        args.push("--fullscreen".into());
    } else if let (Some(w), Some(h)) = (inst.width, inst.height) {
        args.extend(["--width".into(), w.to_string(), "--height".into(), h.to_string()]);
    }
    let quick_play = p.json.to_string().contains("quickPlayMultiplayer");
    let server_target = match quick {
        Some(q) if q.kind == "world" => {
            if quick_play {
                args.extend(["--quickPlaySingleplayer".into(), q.target.clone()]);
            }
            None
        }
        Some(q) if q.kind == "server" => Some(q.target.clone()),
        _ => inst.server.clone(),
    };
    // Join a server right away: quick play on 1.20+, the old --server flag before that
    if let Some(server) = server_target.as_ref().filter(|s| !s.is_empty()) {
        if quick_play {
            let addr = if server.contains(':') { server.clone() } else { format!("{server}:25565") };
            args.extend(["--quickPlayMultiplayer".into(), addr]);
        } else {
            let (host, port) = server.split_once(':').unwrap_or((server.as_str(), "25565"));
            args.extend(["--server".into(), host.to_string(), "--port".into(), port.to_string()]);
        }
    }
    Ok(args)
}

/// What the launcher window does while the game runs: keep | minimize | hide.
fn window_on_start<R: Runtime>(app: &AppHandle<R>, state: &AppState) {
    let mode = state
        .settings()
        .get("onLaunch")
        .and_then(|v| v.as_str())
        .unwrap_or("keep")
        .to_string();
    if let Some(w) = app.get_webview_window("main") {
        match mode.as_str() {
            "minimize" => {
                let _ = w.minimize();
            }
            "hide" => {
                let _ = w.hide();
            }
            _ => {}
        }
    }
}

fn window_on_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        if !w.is_visible().unwrap_or(true) {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

#[tauri::command]
pub async fn launch_instance<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: String,
    quick: Option<QuickPlay>,
) -> Res<()> {
    if state.running.lock().map(|r| r.contains_key(&id)).unwrap_or(false) {
        return Err("Эта сборка уже запущена".into());
    }
    let mut inst = instances::load(&state, &id)?;
    let accounts = auth::load(&state);
    let active = accounts
        .active
        .clone()
        .or_else(|| accounts.accounts.first().map(|a| a.id.clone()))
        .ok_or("Сначала войди в аккаунт")?;
    let account = auth::ensure_fresh(&state, &active).await?;
    if account.needs_name {
        return Err("Сначала придумай ник для Minecraft в профиле".into());
    }
    if account.kind == "offline" && !accounts.accounts.iter().any(|a| a.kind == "msa") {
        return Err("Офлайн-профили работают только вместе с лицензионным аккаунтом".into());
    }

    let game_dir = instances::game_dir(&state, &id);
    std::fs::create_dir_all(&game_dir).ctx("Не удалось создать папку сборки")?;
    let app_e = app.clone();
    let id_e = id.clone();
    let emit: Emit = Arc::new(move |stage: &str, done: usize, total: usize| {
        let _ = app_e.emit(
            "install-progress",
            json!({"instanceId": id_e, "stage": stage, "done": done, "total": total}),
        );
    });
    let auto_update = state
        .settings()
        .get("autoUpdateMods")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if auto_update && inst.loader != "vanilla" {
        emit("updates", 0, 1);
        let _ = crate::content::apply_updates_inner(&state, &id, &[]).await;
    }
    let p = prepare(
        &state,
        &emit,
        &inst.mc_version,
        &inst.loader,
        inst.loader_version.as_deref(),
        &game_dir,
    )
    .await?;
    // first launch of a build: default game settings, packs and shader switched on
    crate::defaults::first_run(&state, &id, &game_dir, &p.classpath);
    // the button pack in the launcher's colours (1.21+); never a reason not to start
    if let Err(e) = crate::gameui::sync_pack(&state, &inst, &p.classpath) {
        eprintln!("AniLauncher theme pack: {e}");
    }
    let ui_config = crate::gameui::prepare(&state, &inst, &account).await;
    let mut args = build_args(&state, &inst, &account, &p, &game_dir, quick.as_ref())?;
    // The companion mod is loaded from the launcher's folder, never from mods/
    match crate::link::jvm_args(&app, &state, &id, &p.mc, &inst.loader, &p.json) {
        Ok(Some(mut extra)) => {
            if let Some(ui) = &ui_config {
                extra.jvm.push(format!("-Danilink.ui={}", ui.to_string_lossy()));
            }
            extra.apply(&mut args)
        }
        Ok(None) => {}
        Err(e) => eprintln!("AniLauncher Link: {e}"),
    }

    let logs = instances::dir(&state, &id).join("logs");
    std::fs::create_dir_all(&logs).ctx("Не удалось создать папку логов")?;
    let log_path = logs.join("latest-launch.log");
    let mut log = std::fs::File::create(&log_path).ctx("Не удалось создать лог")?;

    let java = match inst.java_path.as_ref().filter(|j| !j.is_empty()) {
        Some(own) if Path::new(own).exists() => std::path::PathBuf::from(own),
        Some(own) => return Err(format!("Своя Java не найдена: {own}")),
        None => p.java.clone(),
    };
    let mut cmd = Command::new(&java);
    cmd.args(&args)
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().ctx("Не удалось запустить Java")?;
    let pid = child.id();
    if let Ok(mut r) = state.running.lock() {
        r.insert(id.clone(), pid);
    }
    inst.last_played = Some(now());
    let _ = instances::store(&state, &inst);
    let _ = app.emit("game-started", json!({"instanceId": id, "mc": p.mc}));
    window_on_start(&app, &state);

    // Game output → log file + live console (sent in small batches)
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    for pipe in [
        child.stdout.take().map(|o| Box::new(o) as Box<dyn std::io::Read + Send>),
        child.stderr.take().map(|e| Box::new(e) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(pipe);
            for line in reader.split(b'\n').map_while(Result::ok) {
                let text = String::from_utf8_lossy(&line).trim_end_matches('\r').to_string();
                if tx.send(text).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);
    let app_log = app.clone();
    let id_log = id.clone();
    let pump = std::thread::spawn(move || {
        use std::io::Write;
        let mut batch: Vec<String> = Vec::new();
        let mut last = std::time::Instant::now();
        loop {
            match rx.recv_timeout(std::time::Duration::from_millis(150)) {
                Ok(line) => {
                    let _ = writeln!(log, "{line}");
                    batch.push(line);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if !batch.is_empty() && (batch.len() >= 200 || last.elapsed().as_millis() >= 150) {
                let _ = app_log.emit("game-log", json!({"instanceId": id_log, "lines": std::mem::take(&mut batch)}));
                last = std::time::Instant::now();
            }
        }
        if !batch.is_empty() {
            let _ = app_log.emit("game-log", json!({"instanceId": id_log, "lines": batch}));
        }
        let _ = log.flush();
    });

    let app2 = app.clone();
    let started = now();
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|s| s.code());
        let _ = pump.join();
        let st = app2.state::<AppState>();
        if let Ok(mut r) = st.running.lock() {
            r.remove(&id);
        }
        if let Ok(mut i) = instances::load(&st, &id) {
            i.play_seconds += now().saturating_sub(started);
            let _ = instances::store(&st, &i);
        }
        crate::stats::record(&st, &id, started, now());
        crate::quick::remember_server(&st, &id, &log_path);
        let mut link_disabled = false;
        let (hints, report) = if code.unwrap_or(0) != 0 {
            let (text, report) = crash::collect(&log_path, &instances::game_dir(&st, &id), started);
            // The hidden companion must never be the reason a game doesn't start.
            if crate::link::caused_crash(&text) || report.as_deref().map(crate::link::caused_crash).unwrap_or(false) {
                if let Ok(i) = instances::load(&st, &id) {
                    crate::link::mark_broken(&st, &id, &i.mc_version, &i.loader);
                    link_disabled = true;
                }
            }
            (crash::analyze(&text), report)
        } else {
            (vec![], None)
        };
        window_on_exit(&app2);
        let _ = app2.emit(
            "game-exited",
            json!({"instanceId": id, "code": code, "hints": hints, "crashReport": report, "linkDisabled": link_disabled}),
        );
    });
    Ok(())
}

#[tauri::command(async)]
pub fn running_instances(state: State<'_, AppState>) -> Vec<String> {
    state
        .running
        .lock()
        .map(|r| r.keys().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command(async)]
pub fn stop_instance(state: State<'_, AppState>, id: String) -> Res<()> {
    let pid = state
        .running
        .lock()
        .ok()
        .and_then(|r| r.get(&id).copied())
        .ok_or("Игра не запущена")?;
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"]).creation_flags(CREATE_NO_WINDOW);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("kill");
        c.arg(pid.to_string());
        c
    };
    cmd.status().ctx("Не удалось закрыть игру")?;
    Ok(())
}

/// Bedrock is a Microsoft Store app: we can only start it.
#[tauri::command(async)]
pub fn launch_bedrock() -> Res<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .args(["/C", "start", "", "minecraft:"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ctx("Не удалось запустить Bedrock")?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Bedrock запускается только на Windows".into())
    }
}
