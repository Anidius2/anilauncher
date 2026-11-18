//! Reads the end of the game log (and the newest crash report), explains common crashes
//! and offers one-click fixes: install a missing mod, update or disable a broken one,
//! give the game more memory, go back to the launcher's own Java.

use crate::content::{self, API};
use crate::instances;
use crate::net::get_json;
use crate::state::AppState;
use crate::util::{Ctx, Res};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use tauri::State;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    /// install | update | disable | memory | java
    pub action: String,
    /// mod id for install/update/disable
    pub target: String,
    /// what to show the player
    pub name: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    /// memory | java | missingMod | mixin | duplicate | driver | modError | wrongGame | incompatible
    pub kind: String,
    pub detail: String,
    pub fixes: Vec<Fix>,
}

fn hint(kind: &str, detail: impl Into<String>) -> Hint {
    Hint { kind: kind.into(), detail: detail.into(), fixes: vec![] }
}

fn fix(action: &str, target: &str, name: &str) -> Fix {
    Fix { action: action.into(), target: target.into(), name: name.into() }
}

/// Ids that must never be "disabled" or "installed" as a fix.
fn core_id(id: &str) -> bool {
    matches!(id, "minecraft" | "java" | "fabricloader" | "fabric-loader" | "quilt_loader" | "forge" | "neoforge" | "mixinextras")
        || id.starts_with("fabric-") && id != "fabric-api"
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// `'Name' (id)` or `Name (id)` pairs in a line, in order.
fn named_ids(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')') else { break };
        let id = rest[open + 1..open + close].trim();
        // "Fabric Rendering API (v1) (fabric-rendering-v1)": only the last bracket of a run is the id
        let next_is_bracket = rest[open + close + 1..].trim_start().starts_with('(');
        if valid_id(id) && !next_is_bracket {
            let before = rest[..open].trim_end();
            let name = if let Some(stripped) = before.strip_suffix('\'') {
                stripped.rsplit('\'').next().unwrap_or(id).to_string()
            } else {
                // unquoted: the words after the last separator
                before.rsplit([',', ':']).next().unwrap_or(id).trim().to_string()
            };
            let name = if name.is_empty() { id.to_string() } else { name };
            out.push((name, id.to_string()));
        }
        rest = &rest[open + close + 1..];
    }
    out
}

/// The thing after "of" in "requires ... of X, which is missing": `mod 'Name' (id)` or a bare id.
fn required_mod(line: &str) -> Option<(String, String)> {
    let head = line.split(", which is missing").next()?;
    let after_requires = &head[head.find("requires")?..];
    let of = after_requires.rfind(" of ")?;
    let target = after_requires[of + 4..].trim();
    if let Some(pair) = named_ids(target).into_iter().last() {
        return Some(pair);
    }
    let id = target.trim_start_matches("mod ").trim_matches('\'').trim();
    valid_id(id).then(|| (id.to_string(), id.to_string()))
}

pub fn analyze(text: &str) -> Vec<Hint> {
    let mut out: Vec<Hint> = Vec::new();
    let mut push = |h: Hint| {
        if !out.iter().any(|o| o.kind == h.kind && o.detail == h.detail) && out.len() < 8 {
            out.push(h);
        }
    };
    let mut in_suspected = false;
    for raw in text.lines() {
        let line = raw.trim();
        let lower = line.to_lowercase();

        // Fabric / Quilt: the suggested solution
        if let Some(rest) = line.strip_prefix("- Install ") {
            let id = rest.split([',', ' ']).next().unwrap_or("").trim_end_matches('.');
            let mut h = hint("missingMod", format!("Install {rest}"));
            if valid_id(id) && !core_id(id) {
                h.fixes.push(fix("install", id, id));
            }
            push(h);
        } else if let Some(rest) = line.strip_prefix("- Replace ") {
            let mut h = hint("missingMod", format!("Replace {rest}"));
            if let Some((name, id)) = named_ids(rest).into_iter().next() {
                if !core_id(&id) {
                    h.fixes.push(fix("update", &id, &name));
                    h.fixes.push(fix("disable", &id, &name));
                }
            }
            push(h);
        } else if lower.contains("requires") && lower.contains("which is missing") {
            // "- Mod 'Iris' (iris) 1.7 requires any version of fabric-api, which is missing!"
            let mut h = hint("missingMod", line.trim_start_matches(|c: char| !c.is_alphanumeric()));
            if let Some((name, id)) = required_mod(line) {
                if !core_id(&id) {
                    h.fixes.push(fix("install", &id, &name));
                }
            }
            push(h);
        } else if lower.contains("requires") && (lower.contains("of 'minecraft' (minecraft)") || lower.contains("of minecraft")) && lower.contains("wrong version") {
            // a mod made for another game version
            let mut h = hint("wrongGame", line.trim_start_matches(|c: char| !c.is_alphanumeric()));
            if let Some((name, id)) = named_ids(line).into_iter().next().filter(|(_, id)| !core_id(id)) {
                h.detail = name.clone();
                h.fixes.push(fix("update", &id, &name));
                h.fixes.push(fix("disable", &id, &name));
            }
            push(h);
        } else if lower.contains("is incompatible with") || (lower.contains("conflicts with") && lower.starts_with("- mod")) {
            let pairs: Vec<(String, String)> = named_ids(line).into_iter().filter(|(_, id)| !core_id(id)).collect();
            let mut h = hint("incompatible", line.trim_start_matches(|c: char| !c.is_alphanumeric()));
            if pairs.len() >= 2 {
                h.detail = format!("{} + {}", pairs[0].0, pairs[1].0);
                for (name, id) in pairs.iter().take(2) {
                    h.fixes.push(fix("disable", id, name));
                }
            }
            push(h);
        }

        // Forge / NeoForge: missing mandatory dependencies
        if line.contains("Mod ID: '") && line.contains("Requested by") {
            let mut h = hint("missingMod", line);
            if let Some(id) = line.split("Mod ID: '").nth(1).and_then(|r| r.split('\'').next()) {
                if valid_id(id) && !core_id(id) {
                    h.fixes.push(fix("install", id, id));
                }
            }
            push(h);
        }

        if line.contains("java.lang.OutOfMemoryError") {
            let mut h = hint("memory", "");
            h.fixes.push(fix("memory", "", ""));
            push(h);
        }
        if line.contains("UnsupportedClassVersionError") || line.contains("compiled by a more recent version of the Java Runtime") {
            let mut h = hint("java", "");
            h.fixes.push(fix("java", "", ""));
            push(h);
        }

        if let Some(idx) = line.find("Mixin apply for mod ") {
            let id = line[idx + 20..].split_whitespace().next().unwrap_or("").to_string();
            let mut h = hint("mixin", id.clone());
            if valid_id(&id) && !core_id(&id) {
                h.fixes.push(fix("update", &id, &id));
                h.fixes.push(fix("disable", &id, &id));
            }
            push(h);
        } else if line.contains("MixinApplyError") || line.contains("Mixin apply failed") {
            push(hint("mixin", ""));
        }

        if lower.contains("duplicate mod") || lower.contains("found duplicate") || lower.contains("provided by multiple mods") {
            push(hint("duplicate", line));
        }
        if lower.contains("pixel format not accelerated")
            || line.contains("GLFW error 65542")
            || line.contains("GLFW error 65543")
            || lower.contains("driver does not appear to support opengl")
            || line.contains("Failed to initialize GLFW")
            || lower.contains("could not create context")
        {
            push(hint("driver", ""));
        }

        // Crash report: "Suspected Mod(s): Iris (iris), Sodium (sodium)" — the ids can also follow on indented lines
        if line.starts_with("Suspected Mod") {
            in_suspected = !lower.contains("none");
        } else if in_suspected && !raw.starts_with('\t') && !raw.starts_with(' ') {
            in_suspected = false;
        }
        if in_suspected || line.starts_with("Suspected Mod") {
            for (name, id) in named_ids(line).into_iter().filter(|(_, id)| !core_id(id)).take(2) {
                let mut h = hint("modError", name.clone());
                h.fixes.push(fix("update", &id, &name));
                h.fixes.push(fix("disable", &id, &name));
                push(h);
            }
        }
    }
    out
}

/// Last lines of the launch log plus the newest crash report written after `since`.
pub fn collect(log: &Path, game_dir: &Path, since: u64) -> (String, Option<String>) {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let mut tail = lines[lines.len().saturating_sub(600)..].join("\n");
    let mut report = None;
    let newest = std::fs::read_dir(game_dir.join("crash-reports"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?.modified().ok()?;
            let secs = m.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            (secs >= since).then(|| (secs, e.path()))
        })
        .max_by_key(|(s, _)| *s);
    if let Some((_, path)) = newest {
        if let Ok(r) = std::fs::read_to_string(&path) {
            tail.push('\n');
            tail.push_str(&r);
            report = Some(path.to_string_lossy().to_string());
        }
    }
    (tail, report)
}

/// Finds the Modrinth project for a mod id: the id as a slug, the id with dashes, then a search.
async fn modrinth_project(state: &AppState, id: &str) -> Option<String> {
    let mut slugs = vec![id.to_string()];
    if id.contains('_') {
        slugs.push(id.replace('_', "-"));
    }
    for slug in &slugs {
        if let Ok(v) = get_json::<Value>(&state.http, &format!("{API}/project/{slug}")).await {
            if v.get("project_type").and_then(|t| t.as_str()) == Some("mod") {
                return v.get("id").and_then(|x| x.as_str()).map(String::from);
            }
        }
    }
    // "yet_another_config_lib_v3" -> "yet another config lib": version words only confuse the search
    let words: Vec<&str> = id
        .split(['_', '-'])
        .filter(|w| !w.is_empty() && !w.trim_start_matches('v').chars().all(|c| c.is_ascii_digit()))
        .collect();
    let url = format!("{API}/search?limit=1&facets=%5B%5B%22project_type%3Amod%22%5D%5D&query={}", urlencode(&words.join(" ")));
    let found: Value = get_json(&state.http, &url).await.ok()?;
    found.pointer("/hits/0/project_id").and_then(|x| x.as_str()).map(String::from)
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "+".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Applies one fix from a crash hint. Returns a short message for the player.
#[tauri::command]
pub async fn crash_fix(state: State<'_, AppState>, instance_id: String, action: String, target: String) -> Res<String> {
    let mods_dir = instances::game_dir(&state, &instance_id).join("mods");
    match action.as_str() {
        "install" => {
            let project = modrinth_project(&state, &target)
                .await
                .ok_or_else(|| format!("Мод «{target}» не нашёлся на Modrinth. Найди его вручную во вкладке модов."))?;
            let titles = content::install_content_inner(&state, &instance_id, "mod", &project, None).await?;
            Ok(format!("Установлено: {}", titles.join(", ")))
        }
        "update" => {
            let (file, name) = crate::conflicts::find_mod(&mods_dir, &target).ok_or_else(|| format!("Мод «{target}» не найден в сборке"))?;
            let n = content::apply_updates_inner(&state, &instance_id, &[file]).await?;
            if n == 0 {
                return Err(format!("Для «{name}» нет новой версии. Попробуй отключить его."));
            }
            Ok(format!("«{name}» обновлён"))
        }
        "disable" => {
            let (file, name) = crate::conflicts::find_mod(&mods_dir, &target).ok_or_else(|| format!("Мод «{target}» не найден в сборке"))?;
            std::fs::rename(mods_dir.join(&file), mods_dir.join(format!("{file}.disabled"))).ctx("Не удалось отключить мод")?;
            Ok(format!("«{name}» отключён. Включить обратно можно во вкладке модов."))
        }
        "memory" => {
            let mut inst = instances::load(&state, &instance_id)?;
            let cur = inst.memory_mb.unwrap_or_else(|| state.memory_mb());
            let total = {
                let mut sys = sysinfo::System::new();
                sys.refresh_memory();
                (sys.total_memory() / 1024 / 1024) as u32
            };
            // leave at least 3 GB to Windows and everything else
            let max = total.saturating_sub(3072).max(2048);
            if cur >= max {
                return Err(format!("У сборки уже {} ГБ — больше на этом компьютере давать нельзя. Убери тяжёлые моды или шейдеры.", cur / 1024));
            }
            let new = (cur + 2048).min(max);
            inst.memory_mb = Some(new);
            instances::store(&state, &inst)?;
            Ok(format!("Памяти для сборки: {:.1} ГБ", new as f32 / 1024.0))
        }
        "java" => {
            let mut inst = instances::load(&state, &instance_id)?;
            inst.java_path = None;
            instances::store(&state, &inst)?;
            Ok("Своя Java убрана — лаунчер сам подберёт нужную".into())
        }
        _ => Err("Неизвестное исправление".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_common_problems() {
        let log = "\
[main/ERROR]: Incompatible mods found!
\t- Install fabric-api, any version.
\t- Replace mod 'Sodium' (sodium) 0.5.0 with version 0.6.0 or later.
java.lang.OutOfMemoryError: Java heap space
Caused by: org.spongepowered.asm.mixin.throwables.MixinApplyError: Mixin apply for mod iris failed iris.mixins.json
GLFW error 65542: WGL: The driver does not appear to support OpenGL
";
        let h = analyze(log);
        let kinds: Vec<&str> = h.iter().map(|x| x.kind.as_str()).collect();
        assert!(kinds.contains(&"missingMod"));
        assert!(kinds.contains(&"memory"));
        assert!(kinds.contains(&"driver"));
        assert!(h.iter().any(|x| x.kind == "mixin" && x.detail == "iris"));
        assert_eq!(h.iter().filter(|x| x.kind == "missingMod").count(), 2);
        let install = h.iter().find(|x| x.detail.starts_with("Install")).unwrap();
        assert_eq!(install.fixes, vec![fix("install", "fabric-api", "fabric-api")]);
        let replace = h.iter().find(|x| x.detail.starts_with("Replace")).unwrap();
        assert_eq!(replace.fixes[0], fix("update", "sodium", "Sodium"));
        assert!(h.iter().find(|x| x.kind == "memory").unwrap().fixes[0].action == "memory");
        assert!(h.iter().find(|x| x.kind == "mixin").unwrap().fixes.iter().any(|f| f.action == "disable" && f.target == "iris"));
    }

    #[test]
    fn new_fabric_format() {
        let log = "\
Incompatible mods found!
More details:
\t - Mod 'Iris' (iris) 1.8.0 requires any version of mod 'Sodium' (sodium), which is missing!
\t - Mod 'Old Thing' (oldthing) 1.0 requires version 1.20.1 of 'Minecraft' (minecraft), but only the wrong version is present: 1.21.11!
\t - Mod 'Alpha' (alpha) 2.0 is incompatible with any version of mod 'Beta Mod' (beta_mod), but a matching version is present: 1.0!
\t - Mod 'Gamma' (gamma) 1.0 requires any version of cloth-config, which is missing!
";
        let h = analyze(log);
        let miss: Vec<&Fix> = h.iter().filter(|x| x.kind == "missingMod").flat_map(|x| &x.fixes).collect();
        assert!(miss.contains(&&fix("install", "sodium", "Sodium")));
        assert!(miss.contains(&&fix("install", "cloth-config", "cloth-config")));
        let wrong = h.iter().find(|x| x.kind == "wrongGame").unwrap();
        assert_eq!(wrong.detail, "Old Thing");
        assert!(wrong.fixes.contains(&fix("disable", "oldthing", "Old Thing")));
        let inc = h.iter().find(|x| x.kind == "incompatible").unwrap();
        assert_eq!(inc.detail, "Alpha + Beta Mod");
        assert_eq!(inc.fixes.len(), 2);
    }

    #[test]
    fn suspected_mods_from_crash_report() {
        let report = "\
-- Head --
Thread: Render thread
Suspected Mods: Iris (iris), Fabric Rendering API (v1) (fabric-rendering-v1)
Stacktrace:
";
        let h = analyze(report);
        let sus: Vec<&Hint> = h.iter().filter(|x| x.kind == "modError").collect();
        assert_eq!(sus.len(), 1, "{h:?}");
        assert_eq!(sus[0].detail, "Iris");
        assert!(sus[0].fixes.contains(&fix("disable", "iris", "Iris")));
        assert!(analyze("Suspected Mods: NONE").is_empty());
    }

    #[test]
    fn never_offers_to_touch_core_parts() {
        let h = analyze("\t- Install minecraft, version 1.21.\n\t - Mod 'X' (x) requires any version of fabricloader, which is missing!");
        assert!(h.iter().all(|x| x.fixes.is_empty()), "{h:?}");
    }
    #[test]
    fn real_fabric_loader_output() {
        // captured from Fabric Loader 0.19 with a mod that needs sodium, cloth-config and Minecraft 1.20.1
        let log = "A potential solution has been determined, this may resolve your problem:
	 - Replace mod 'Needy Test' (testneedy) 1.0 with any version that is compatible with:
		 - minecraft 1.21.11
More details:
	 - Mod 'Needy Test' (testneedy) 1.0 requires version 0.6.0 or later of sodium, which is missing!
	 - Mod 'Needy Test' (testneedy) 1.0 requires any version of cloth-config, which is missing!
	 - Mod 'Needy Test' (testneedy) 1.0 requires version 1.20.1 of 'Minecraft' (minecraft), but only the wrong version is present: 1.21.11!
";
        let h = analyze(log);
        let all: Vec<&Fix> = h.iter().flat_map(|x| &x.fixes).collect();
        assert!(all.contains(&&fix("install", "sodium", "sodium")), "{h:#?}");
        assert!(all.contains(&&fix("install", "cloth-config", "cloth-config")));
        assert!(all.contains(&&fix("disable", "testneedy", "Needy Test")));
        assert!(h.iter().any(|x| x.kind == "wrongGame" && x.detail == "Needy Test"));
        assert!(all.iter().all(|f| f.target != "minecraft"));
    }
}

#[cfg(test)]
mod live {
    /// cargo test --lib crash::live -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn fixes_work_on_a_real_build() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-crashfix");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(crate::state::AppState::with_root(root.clone()));
        let st = app.state::<crate::state::AppState>();
        let inst = crate::instances::Instance {
            id: "fixme".into(),
            name: "Fix me".into(),
            mc_version: "1.21.11".into(),
            loader: "fabric".into(),
            java_path: Some("C:/old/java.exe".into()),
            ..Default::default()
        };
        crate::instances::store(&st, &inst).unwrap();
        std::fs::create_dir_all(crate::instances::game_dir(&st, "fixme").join("mods")).unwrap();

        for id in ["sodium", "cloth-config", "yet_another_config_lib_v3"] {
            println!("{id} -> {:?}", super::modrinth_project(&st, id).await);
        }
        let msg = super::crash_fix(st.clone(), "fixme".into(), "install".into(), "cloth-config".into()).await.unwrap();
        println!("{msg}");
        let mods = crate::instances::game_dir(&st, "fixme").join("mods");
        let list = |d: &std::path::Path| std::fs::read_dir(d).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect::<Vec<_>>();
        println!("{:?}", list(&mods));
        assert!(list(&mods).iter().any(|f| f.starts_with("cloth-config") && f.ends_with(".jar")));

        let msg = super::crash_fix(st.clone(), "fixme".into(), "disable".into(), "cloth-config".into()).await.unwrap();
        println!("{msg}");
        assert!(list(&mods).iter().any(|f| f.starts_with("cloth-config") && f.ends_with(".jar.disabled")));

        println!("{:?}", super::crash_fix(st.clone(), "fixme".into(), "update".into(), "sodium".into()).await);
        println!("{}", super::crash_fix(st.clone(), "fixme".into(), "memory".into(), "".into()).await.unwrap());
        println!("{}", super::crash_fix(st.clone(), "fixme".into(), "java".into(), "".into()).await.unwrap());
        let after = crate::instances::load(&st, "fixme").unwrap();
        assert!(after.java_path.is_none());
        assert!(after.memory_mb.unwrap() > 4096);
    }
}
