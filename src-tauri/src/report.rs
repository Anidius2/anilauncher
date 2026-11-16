//! Bug report in one zip: what the launcher, the computer and the build look like, plus the logs
//! and crash reports. Login tokens are never included and are masked if they appear in a log.

use crate::instances;
use crate::state::AppState;
use crate::util::{now, read_json, Ctx, Res};
use regex::Regex;
use serde_json::Value;
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::State;

const MAX_LOG: u64 = 4 * 1024 * 1024;

/// Masks secrets and the Windows user name in any text that goes into the report.
pub fn scrub(text: &str) -> String {
    static RES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    let res = RES.get_or_init(|| {
        vec![
            (Regex::new(r"(--accessToken|--clientId|--xuid|-Danilink\.token=)(\s*|=)?[^\s,\]]+").unwrap(), "$1 <скрыто>"),
            (Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}").unwrap(), "<токен скрыт>"),
            (Regex::new(r#"(?i)("(?:access|refresh|mc|xbl|xsts)_?token"\s*:\s*")[^"]*"#).unwrap(), "$1<скрыто>"),
            (Regex::new(r"(?i)([A-Z]:[\\/]+Users[\\/]+)[^\\/\s]+").unwrap(), "$1<пользователь>"),
            (Regex::new(r"(/home/)[^/\s]+").unwrap(), "$1<пользователь>"),
        ]
    });
    let mut out = text.to_string();
    for (re, with) in res {
        out = re.replace_all(&out, *with).into_owned();
    }
    out
}

/// "2026-11-16 09:35 UTC" without pulling in a date library.
fn date_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let (h, m) = ((secs % 86_400) / 3600, (secs % 3600) / 60);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if mo <= 2 { 1 } else { 0 };
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02} UTC")
}

/// The last `MAX_LOG` bytes of a text file, scrubbed.
fn read_tail(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    if len > MAX_LOG {
        f.seek(SeekFrom::Start(len - MAX_LOG)).ok()?;
    }
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(scrub(&String::from_utf8_lossy(&buf)))
}

fn newest(dir: &Path, n: usize, filter: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut v: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| filter(&e.file_name().to_string_lossy()))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().take(n).map(|(_, p)| p).collect()
}

fn system_info(state: &AppState) -> String {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let mut s = String::new();
    let _ = writeln!(s, "AniLauncher {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(s, "Время отчёта: {}", date_utc(now()));
    let _ = writeln!(
        s,
        "Система: {} {} ({})",
        sysinfo::System::name().unwrap_or_default(),
        sysinfo::System::os_version().unwrap_or_default(),
        std::env::consts::ARCH
    );
    let cpu = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    let _ = writeln!(s, "Процессор: {} ({} потоков)", cpu, sys.cpus().len());
    let _ = writeln!(s, "Память: {} МБ всего, {} МБ свободно", sys.total_memory() / 1024 / 1024, sys.available_memory() / 1024 / 1024);
    let settings = state.settings();
    let _ = writeln!(
        s,
        "Память для игры по умолчанию: {} МБ; тема: {}; язык: {}; AniLauncher Link: {}",
        settings.get("memoryMb").and_then(|v| v.as_u64()).unwrap_or(4096),
        settings.get("theme").and_then(|v| v.as_str()).unwrap_or("?"),
        settings.get("lang").and_then(|v| v.as_str()).unwrap_or("?"),
        if settings.get("companion").and_then(|v| v.as_bool()).unwrap_or(true) { "вкл" } else { "выкл" },
    );
    let accounts: Value = read_json(&state.root.join("accounts.json")).unwrap_or(Value::Null);
    let kinds: Vec<String> = accounts
        .get("accounts")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().map(|x| x.get("kind").and_then(|k| k.as_str()).unwrap_or("?").to_string()).collect())
        .unwrap_or_default();
    let _ = writeln!(s, "Аккаунты: {} ({})", kinds.len(), kinds.join(", "));
    s
}

fn instance_info(state: &AppState, id: &str) -> Option<(String, Value)> {
    let inst: Value = read_json(&instances::dir(state, id).join("instance.json"))?;
    let g = |k: &str| inst.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mods: Vec<String> = std::fs::read_dir(instances::game_dir(state, id).join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            format!("{} ({} КБ)", e.file_name().to_string_lossy(), size / 1024)
        })
        .collect();
    let mut s = String::new();
    let _ = writeln!(s, "Сборка «{}» [{}]", g("name"), id);
    let _ = writeln!(s, "  Minecraft {} · {} {}", g("mcVersion"), g("loader"), g("loaderVersion"));
    let _ = writeln!(
        s,
        "  Память: {}; свои аргументы Java: {}; своя Java: {}",
        inst.get("memoryMb").and_then(|v| v.as_u64()).map(|m| format!("{m} МБ")).unwrap_or_else(|| "по умолчанию".into()),
        if g("jvmArgs").is_empty() { "нет".to_string() } else { g("jvmArgs") },
        if g("javaPath").is_empty() { "нет" } else { "да" },
    );
    let _ = writeln!(s, "  Моды в папке mods: {}", mods.len());
    for m in &mods {
        let _ = writeln!(s, "    {m}");
    }
    Some((s, inst))
}

fn add_text(zip: &mut zip::ZipWriter<std::fs::File>, name: &str, text: &str) -> Res<()> {
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, opts).ctx("Не удалось записать отчёт")?;
    zip.write_all(text.as_bytes()).ctx("Не удалось записать отчёт")
}

/// Everything about one build: logs, crash reports, installer log, mod list.
fn add_instance(zip: &mut zip::ZipWriter<std::fs::File>, state: &AppState, id: &str, summary: &mut String) -> Res<()> {
    let Some((text, inst)) = instance_info(state, id) else { return Ok(()) };
    summary.push_str(&text);
    summary.push('\n');
    let p = format!("{id}/");
    let dir = instances::dir(state, id);
    let game = instances::game_dir(state, id);
    if let Some(t) = read_tail(&dir.join("logs").join("latest-launch.log")) {
        add_text(zip, &format!("{p}launch-output.log"), &t)?;
    }
    for name in ["latest.log", "debug.log"] {
        if let Some(t) = read_tail(&game.join("logs").join(name)) {
            add_text(zip, &format!("{p}game-{name}"), &t)?;
        }
    }
    for f in newest(&game.join("crash-reports"), 3, |n| n.ends_with(".txt")) {
        if let Some(t) = read_tail(&f) {
            add_text(zip, &format!("{p}crash-reports/{}", f.file_name().unwrap_or_default().to_string_lossy()), &t)?;
        }
    }
    for f in newest(&game, 2, |n| n.starts_with("hs_err_pid") && n.ends_with(".log")) {
        if let Some(t) = read_tail(&f) {
            add_text(zip, &format!("{p}{}", f.file_name().unwrap_or_default().to_string_lossy()), &t)?;
        }
    }
    if let Some(c) = read_tail(&dir.join("content.json")) {
        add_text(zip, &format!("{p}content.json"), &c)?;
    }
    add_text(zip, &format!("{p}instance.json"), &scrub(&serde_json::to_string_pretty(&inst).unwrap_or_default()))?;
    // the Forge/NeoForge installer log for this build's loader, if any
    let loader = inst.get("loader").and_then(|v| v.as_str()).unwrap_or("");
    if loader == "forge" || loader == "neoforge" {
        let ver = inst.get("loaderVersion").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let exact = format!("{loader}-{ver}.log");
        for f in newest(&state.root.join("cache").join("installers"), 1, |n| {
            if ver.is_empty() { n.starts_with(&format!("{loader}-")) && n.ends_with(".log") } else { n == exact }
        }) {
            if let Some(t) = read_tail(&f) {
                add_text(zip, &format!("{p}installer.log"), &t)?;
            }
        }
    }
    Ok(())
}

/// Writes the report zip. With `instance_id` the report is about that build; without it, a short
/// overview of every build plus their latest launch output.
#[tauri::command]
pub async fn bug_report(state: State<'_, AppState>, dest: String, instance_id: Option<String>, note: Option<String>) -> Res<()> {
    let dest = PathBuf::from(dest);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&dest).ctx("Не удалось создать отчёт")?);
    let mut summary = String::new();
    summary.push_str(&system_info(&state));
    summary.push('\n');
    if let Some(n) = note.filter(|n| !n.trim().is_empty()) {
        let _ = writeln!(summary, "Что случилось (со слов игрока):\n{}\n", scrub(n.trim()));
    }
    let ids: Vec<String> = match &instance_id {
        Some(id) => vec![id.clone()],
        None => std::fs::read_dir(state.instances())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().join("instance.json").is_file())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
    };
    for id in &ids {
        add_instance(&mut zip, &state, id, &mut summary)?;
    }
    for f in ["disabled.json"] {
        if let Some(t) = read_tail(&state.root.join("companion").join(f)) {
            add_text(&mut zip, &format!("launcher/companion-{f}"), &t)?;
        }
    }
    // settings without anything personal
    let mut settings = state.settings();
    if let Some(o) = settings.as_object_mut() {
        for k in ["clientId", "birthday", "streamerNote", "dataDir"] {
            o.remove(k);
        }
        if let Some(p) = o.get_mut("profile").and_then(|p| p.as_object_mut()) {
            p.remove("avatar");
        }
    }
    add_text(&mut zip, "launcher/settings.json", &scrub(&serde_json::to_string_pretty(&settings).unwrap_or_default()))?;
    add_text(&mut zip, "README.txt", &summary)?;
    zip.finish().ctx("Не удалось записать отчёт")?;
    Ok(())
}

/// Shows a file the launcher just saved in Explorer.
#[tauri::command]
pub fn reveal_file(app: tauri::AppHandle, path: String) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(path).ctx("Не удалось открыть папку")
}

#[cfg(test)]
mod tests {
    use super::scrub;

    #[test]
    fn dates() {
        assert_eq!(super::date_utc(1_794_822_064), "2026-11-16 09:41 UTC");
    }

    #[test]
    fn hides_secrets() {
        let s = scrub("args [--username, Ani, --accessToken, eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk, --xuid, 123] C:\\Users\\Anidius\\AppData -Danilink.token=abc123");
        assert!(!s.contains("eyJhbGci"), "{s}");
        assert!(!s.contains("Anidius\\AppData"), "{s}");
        assert!(!s.contains("abc123"), "{s}");
        assert!(s.contains("--username, Ani"), "{s}");
        let j = scrub(r#"{"refresh_token": "M.C123", "name": "x"}"#);
        assert!(!j.contains("M.C123"), "{j}");
    }
}
