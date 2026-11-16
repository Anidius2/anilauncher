//! Checks a build's mods before launch: missing dependencies, conflicts, duplicates,
//! mods for the wrong loader or game version. Works offline from the mods' own metadata
//! (fabric.mod.json / quilt.mod.json) and adds Modrinth's compatibility data when online.

use crate::content::{load_meta, modrinth_loaders, sha1_file, API};
use crate::instances;
use crate::state::AppState;
use crate::util::Res;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use tauri::State;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// "error" (the game will most likely not start) or "warning"
    pub severity: String,
    /// duplicate | missingDep | versionMismatch | breaks | conflicts | wrongLoader | wrongGame | incompatible
    pub kind: String,
    pub mod_name: String,
    /// file to disable for a quick fix
    pub mod_file: String,
    pub other: String,
    pub detail: String,
}

// ---------- versions ----------

fn parse_version(v: &str) -> (Vec<u64>, bool) {
    let core = v.split('+').next().unwrap_or(v);
    let (main, pre) = match core.split_once('-') {
        Some((m, _)) => (m, true),
        None => (core, false),
    };
    let nums = main
        .split('.')
        .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0))
        .collect();
    (nums, pre)
}

fn cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (na, pa) = parse_version(a);
    let (nb, pb) = parse_version(b);
    for i in 0..na.len().max(nb.len()) {
        let x = na.get(i).copied().unwrap_or(0);
        let y = nb.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    // a pre-release is older than the release with the same numbers
    pb.cmp(&pa)
}

fn one_predicate(have: &str, pred: &str) -> bool {
    let p = pred.trim();
    if p.is_empty() || p == "*" {
        return true;
    }
    for op in [">=", "<=", ">", "<", "=", "~", "^"] {
        if let Some(v) = p.strip_prefix(op) {
            let v = v.trim();
            let o = cmp(have, v);
            return match op {
                ">=" => o != std::cmp::Ordering::Less,
                "<=" => o != std::cmp::Ordering::Greater,
                ">" => o == std::cmp::Ordering::Greater,
                "<" => o == std::cmp::Ordering::Less,
                "=" => o == std::cmp::Ordering::Equal,
                "~" | "^" => {
                    let (hv, _) = parse_version(have);
                    let (vv, _) = parse_version(v);
                    let keep = if op == "~" { 2 } else { 1 };
                    o != std::cmp::Ordering::Less && hv.iter().take(keep).eq(vv.iter().take(keep))
                }
                _ => true,
            };
        }
    }
    if p.contains(".x") || p.ends_with(".*") {
        let prefix = p.trim_end_matches(".x").trim_end_matches(".*");
        return have == prefix || have.starts_with(&format!("{prefix}."));
    }
    cmp(have, p) == std::cmp::Ordering::Equal
}

/// Fabric version range: a string (space-separated = AND) or a list (OR).
pub fn matches(have: &str, range: &Value) -> bool {
    match range {
        Value::String(s) => s.split_whitespace().all(|p| one_predicate(have, p)) || s.trim().is_empty(),
        Value::Array(a) => a.is_empty() || a.iter().any(|r| matches(have, r)),
        _ => true,
    }
}

fn range_text(range: &Value) -> String {
    match range {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(range_text).collect::<Vec<_>>().join(" | "),
        _ => String::new(),
    }
}

// ---------- reading mod metadata ----------

#[derive(Debug, Default, Clone)]
struct ModInfo {
    id: String,
    name: String,
    version: String,
    file: String,
    provides: Vec<(String, String)>,
    depends: Vec<(String, Value)>,
    breaks: Vec<(String, Value)>,
    conflicts: Vec<(String, Value)>,
    /// bundled inside another jar
    nested: bool,
}

fn obj_pairs(v: Option<&Value>) -> Vec<(String, Value)> {
    v.and_then(|x| x.as_object())
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

/// Quilt lists dependencies as strings or {id, versions}.
fn quilt_pairs(v: Option<&Value>) -> Vec<(String, Value)> {
    v.and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|d| match d {
                    Value::String(id) => Some((id.clone(), json!("*"))),
                    Value::Object(o) => {
                        if o.get("optional").and_then(|x| x.as_bool()).unwrap_or(false) {
                            return None;
                        }
                        let id = o.get("id")?.as_str()?.to_string();
                        Some((id, o.get("versions").cloned().unwrap_or(json!("*"))))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_zip_entry<R: Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, name: &str) -> Option<Vec<u8>> {
    let mut e = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    e.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Reads a jar's metadata plus the mods bundled inside it (jar-in-jar).
fn read_mods<R: Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, file: &str, depth: u8, out: &mut Vec<ModInfo>, top: bool) {
    let mut info = ModInfo { file: file.to_string(), ..Default::default() };
    let mut nested: Vec<String> = Vec::new();
    if let Some(raw) = read_zip_entry(zip, "fabric.mod.json") {
        // Some mods have stray control characters in their JSON
        let text = String::from_utf8_lossy(&raw).replace(['\n', '\r', '\t'], " ");
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            info.id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            info.name = v.get("name").and_then(|x| x.as_str()).unwrap_or(&info.id).to_string();
            info.version = v.get("version").and_then(|x| x.as_str()).unwrap_or("0").to_string();
            info.provides = v
                .get("provides")
                .and_then(|p| p.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| (s.to_string(), info.version.clone()))).collect())
                .unwrap_or_default();
            info.depends = obj_pairs(v.get("depends"));
            info.breaks = obj_pairs(v.get("breaks"));
            info.conflicts = obj_pairs(v.get("conflicts"));
            nested = v
                .get("jars")
                .and_then(|j| j.as_array())
                .map(|a| a.iter().filter_map(|x| x.get("file").and_then(|f| f.as_str()).map(String::from)).collect())
                .unwrap_or_default();
        }
    } else if let Some(raw) = read_zip_entry(zip, "quilt.mod.json") {
        if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
            let ql = v.get("quilt_loader").cloned().unwrap_or(Value::Null);
            info.id = ql.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
            info.version = ql.get("version").and_then(|x| x.as_str()).unwrap_or("0").to_string();
            info.name = ql.pointer("/metadata/name").and_then(|x| x.as_str()).unwrap_or(&info.id).to_string();
            info.provides = ql
                .get("provides")
                .and_then(|p| p.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| match x {
                            Value::String(s) => Some((s.clone(), info.version.clone())),
                            Value::Object(o) => Some((o.get("id")?.as_str()?.to_string(), info.version.clone())),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            info.depends = quilt_pairs(ql.get("depends"));
            info.breaks = quilt_pairs(ql.get("breaks"));
            nested = ql
                .get("jars")
                .and_then(|j| j.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
        }
    }
    if info.id.is_empty() {
        return;
    }
    if depth < 3 {
        for n in nested {
            if let Some(bytes) = read_zip_entry(zip, &n) {
                if let Ok(mut inner) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) {
                    read_mods(&mut inner, file, depth + 1, out, false);
                }
            }
        }
    }
    // Bundled mods only provide ids; their own dependencies are their parent's business
    info.nested = !top;
    if !top {
        info.depends.clear();
        info.breaks.clear();
        info.conflicts.clear();
    }
    out.push(info);
}

fn scan_folder(dir: &Path) -> (Vec<ModInfo>, Vec<(String, String)>) {
    let mut mods = Vec::new();
    let mut files = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jar") || !e.path().is_file() {
            continue;
        }
        files.push((name.clone(), e.path().to_string_lossy().to_string()));
        if let Ok(f) = std::fs::File::open(e.path()) {
            if let Ok(mut zip) = zip::ZipArchive::new(f) {
                read_mods(&mut zip, &name, 0, &mut mods, true);
            }
        }
    }
    (mods, files)
}

/// The offline part of the check: works for any Fabric/Quilt mod, even ones not on Modrinth.
fn check_local(mods: &[ModInfo], mc: &str, loader: &str) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut provided: HashMap<String, String> = HashMap::new();
    // Provided by the loader itself (Fabric Loader 0.15+ bundles MixinExtras)
    for b in ["java", "fabricloader", "fabric-loader", "quilt_loader", "quilt_base", "mixinextras"] {
        provided.insert(b.into(), "999".into());
    }
    provided.insert("minecraft".into(), mc.into());
    let top: Vec<&ModInfo> = mods.iter().filter(|m| !m.nested).collect();
    for m in mods {
        provided.entry(m.id.clone()).or_insert_with(|| m.version.clone());
        for (p, v) in &m.provides {
            provided.entry(p.clone()).or_insert_with(|| v.clone());
        }
    }
    // Duplicates: the same mod id from two different top-level files
    let mut by_id: HashMap<&str, Vec<&ModInfo>> = HashMap::new();
    for m in &top {
        by_id.entry(m.id.as_str()).or_default().push(m);
    }
    for (id, list) in &by_id {
        let files: HashSet<&str> = list.iter().map(|m| m.file.as_str()).collect();
        if files.len() > 1 {
            let mut f: Vec<&str> = files.into_iter().collect();
            f.sort();
            out.push(Problem {
                severity: "error".into(),
                kind: "duplicate".into(),
                mod_name: list[0].name.clone(),
                mod_file: f[1].to_string(),
                other: f[0].to_string(),
                detail: id.to_string(),
            });
        }
    }
    let quilt = loader == "quilt";
    for m in mods.iter().filter(|m| !m.depends.is_empty() || !m.breaks.is_empty() || !m.conflicts.is_empty()) {
        for (dep, range) in &m.depends {
            if quilt && dep.starts_with("quilted_fabric") {
                continue;
            }
            match provided.get(dep) {
                None => out.push(Problem {
                    severity: "error".into(),
                    kind: "missingDep".into(),
                    mod_name: m.name.clone(),
                    mod_file: m.file.clone(),
                    other: dep.clone(),
                    detail: range_text(range),
                }),
                Some(have) if dep == "minecraft" => {
                    if !matches(have, range) {
                        out.push(Problem {
                            severity: "warning".into(),
                            kind: "wrongGame".into(),
                            mod_name: m.name.clone(),
                            mod_file: m.file.clone(),
                            other: mc.to_string(),
                            detail: range_text(range),
                        });
                    }
                }
                Some(have) if have != "999" && !matches(have, range) => out.push(Problem {
                    severity: "error".into(),
                    kind: "versionMismatch".into(),
                    mod_name: m.name.clone(),
                    mod_file: m.file.clone(),
                    other: dep.clone(),
                    detail: format!("{} → {}", have, range_text(range)),
                }),
                _ => {}
            }
        }
        for (kind, severity, list) in [("breaks", "error", &m.breaks), ("conflicts", "warning", &m.conflicts)] {
            for (other, range) in list {
                if let Some(have) = provided.get(other) {
                    if other != &m.id && matches(have, range) {
                        let other_name = mods.iter().find(|x| &x.id == other).map(|x| x.name.clone()).unwrap_or(other.clone());
                        out.push(Problem {
                            severity: severity.into(),
                            kind: kind.into(),
                            mod_name: m.name.clone(),
                            mod_file: m.file.clone(),
                            other: other_name,
                            detail: range_text(range),
                        });
                    }
                }
            }
        }
    }
    out
}

/// Modrinth knows a mod's loaders, game versions and declared incompatibilities.
async fn check_modrinth(state: &AppState, instance_id: &str, files: &[(String, String)], mc: &str, loader: &str) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut by_hash: HashMap<String, String> = HashMap::new();
    for (name, path) in files {
        if let Some(h) = sha1_file(Path::new(path)) {
            by_hash.insert(h, name.clone());
        }
    }
    if by_hash.is_empty() {
        return out;
    }
    let req = state
        .http
        .post(format!("{API}/version_files"))
        .timeout(std::time::Duration::from_secs(8))
        .json(&json!({"hashes": by_hash.keys().collect::<Vec<_>>(), "algorithm": "sha1"}))
        .send()
        .await;
    let found: Value = match req {
        Ok(r) => r.json().await.unwrap_or(Value::Null),
        Err(_) => return out,
    };
    let meta = load_meta(state, instance_id);
    let title_of = |file: &str| {
        meta.entries
            .iter()
            .find(|e| e.filename == file)
            .and_then(|e| e.title.clone())
            .unwrap_or_else(|| file.trim_end_matches(".jar").to_string())
    };
    let mut installed: HashMap<String, String> = HashMap::new();
    for (hash, file) in &by_hash {
        if let Some(v) = found.get(hash) {
            if let Some(pid) = v.get("project_id").and_then(|x| x.as_str()) {
                installed.insert(pid.to_string(), file.clone());
            }
        }
    }
    let wanted = modrinth_loaders(loader);
    for (hash, file) in &by_hash {
        let Some(v) = found.get(hash) else { continue };
        let loaders: Vec<&str> = v.get("loaders").and_then(|l| l.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        if !wanted.is_empty() && !loaders.is_empty() && !loaders.iter().any(|l| wanted.contains(l)) {
            out.push(Problem {
                severity: "error".into(),
                kind: "wrongLoader".into(),
                mod_name: title_of(file),
                mod_file: file.clone(),
                other: loaders.join(", "),
                detail: String::new(),
            });
        }
        let games: Vec<&str> = v.get("game_versions").and_then(|l| l.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        if !games.is_empty() && !games.contains(&mc) {
            out.push(Problem {
                severity: "warning".into(),
                kind: "wrongGame".into(),
                mod_name: title_of(file),
                mod_file: file.clone(),
                other: mc.to_string(),
                detail: games.iter().rev().take(4).copied().collect::<Vec<_>>().join(", "),
            });
        }
        for dep in v.get("dependencies").and_then(|d| d.as_array()).into_iter().flatten() {
            let kind = dep.get("dependency_type").and_then(|x| x.as_str()).unwrap_or("");
            let Some(pid) = dep.get("project_id").and_then(|x| x.as_str()) else { continue };
            if kind == "incompatible" {
                if let Some(other_file) = installed.get(pid) {
                    out.push(Problem {
                        severity: "error".into(),
                        kind: "incompatible".into(),
                        mod_name: title_of(file),
                        mod_file: file.clone(),
                        other: title_of(other_file),
                        detail: String::new(),
                    });
                }
            }
        }
    }
    out
}

pub async fn check_inner(state: &AppState, instance_id: &str) -> Res<Vec<Problem>> {
    let inst = instances::load(state, instance_id)?;
    if inst.loader == "vanilla" {
        return Ok(vec![]);
    }
    let dir = instances::game_dir(state, instance_id).join("mods");
    let (mods, files) = scan_folder(&dir);
    let mut out = if inst.loader == "fabric" || inst.loader == "quilt" {
        check_local(&mods, &inst.mc_version, &inst.loader)
    } else {
        vec![]
    };
    for p in check_modrinth(state, instance_id, &files, &inst.mc_version, &inst.loader).await {
        // Skip what the local check already reported for the same file
        if !out.iter().any(|o| o.mod_file == p.mod_file && o.kind == p.kind) {
            out.push(p);
        }
    }
    out.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.mod_name.cmp(&b.mod_name)));
    out.dedup();
    Ok(out)
}

#[tauri::command]
pub async fn check_mods(state: State<'_, AppState>, instance_id: String) -> Res<Vec<Problem>> {
    check_inner(&state, &instance_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ranges() {
        assert!(matches("0.5.8", &json!(">=0.5.0")));
        assert!(!matches("0.4.9", &json!(">=0.5.0")));
        assert!(matches("1.21.1", &json!("1.21.x")));
        assert!(matches("1.21.1", &json!("~1.21")));
        assert!(!matches("1.22", &json!("~1.21")));
        assert!(matches("2.3.0", &json!("^2.1")));
        assert!(matches("1.0.0", &json!([">=2", "1.0.0"])));
        assert!(matches("1.0.0", &json!(">=0.9 <1.1")));
        assert!(!matches("1.2.0", &json!(">=0.9 <1.1")));
        assert!(matches("0.16.5", &json!("*")));
        assert!(matches("1.0.0-beta.2", &json!("<1.0.0")));
    }

    #[test]
    fn local_checks() {
        let a = ModInfo {
            id: "a".into(), name: "Mod A".into(), version: "1.0".into(), file: "a.jar".into(),
            depends: vec![("b".into(), json!(">=2.0")), ("c".into(), json!("*")), ("minecraft".into(), json!("1.20.x"))],
            breaks: vec![("d".into(), json!("*"))],
            ..Default::default()
        };
        let b = ModInfo { id: "b".into(), name: "Mod B".into(), version: "1.5".into(), file: "b.jar".into(), ..Default::default() };
        let d = ModInfo { id: "d".into(), name: "Mod D".into(), version: "3".into(), file: "d.jar".into(), ..Default::default() };
        let d2 = ModInfo { id: "d".into(), name: "Mod D".into(), version: "3".into(), file: "d-copy.jar".into(), ..Default::default() };
        let p = check_local(&[a, b, d, d2], "1.21.1", "fabric");
        let kinds: Vec<&str> = p.iter().map(|x| x.kind.as_str()).collect();
        assert!(kinds.contains(&"versionMismatch"), "{kinds:?}");
        assert!(kinds.contains(&"missingDep"));
        assert!(kinds.contains(&"breaks"));
        assert!(kinds.contains(&"duplicate"));
        assert!(kinds.contains(&"wrongGame"));
    }
}
