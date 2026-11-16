//! Reads the end of the game log (and the newest crash report) and explains common crashes.

use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    /// memory | java | missingMod | mixin | duplicate | driver | modError
    pub kind: String,
    pub detail: String,
}

fn hint(kind: &str, detail: impl Into<String>) -> Hint {
    Hint { kind: kind.into(), detail: detail.into() }
}

pub fn analyze(text: &str) -> Vec<Hint> {
    let mut out: Vec<Hint> = Vec::new();
    let mut push = |h: Hint| {
        if !out.contains(&h) && out.len() < 8 {
            out.push(h);
        }
    };
    for raw in text.lines() {
        let line = raw.trim();
        // Fabric / Quilt: missing or wrong-version dependencies
        if let Some(rest) = line.strip_prefix("- Install ").or_else(|| line.strip_prefix("- Replace ")) {
            let verb = if line.starts_with("- Install") { "Install " } else { "Replace " };
            push(hint("missingMod", format!("{verb}{rest}")));
        }
        // Forge / NeoForge: missing mandatory dependencies
        if line.contains("Mod ID: '") && line.contains("Requested by") {
            push(hint("missingMod", line));
        }
        if line.contains("requires") && line.contains("which is missing") {
            push(hint("missingMod", line.trim_start_matches(|c: char| !c.is_alphanumeric())));
        }
        if line.contains("java.lang.OutOfMemoryError") {
            push(hint("memory", ""));
        }
        if line.contains("UnsupportedClassVersionError") || line.contains("compiled by a more recent version of the Java Runtime") {
            push(hint("java", ""));
        }
        if let Some(idx) = line.find("Mixin apply for mod ") {
            let m = line[idx + 20..].split_whitespace().next().unwrap_or("").to_string();
            push(hint("mixin", m));
        } else if line.contains("MixinApplyError") || line.contains("Mixin apply failed") {
            push(hint("mixin", ""));
        }
        let lower = line.to_lowercase();
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
        if line.starts_with("Suspected Mod") || line.starts_with("Suspected Mods:") {
            push(hint("modError", line));
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
    }
}
