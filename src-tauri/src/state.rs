use crate::util::{read_json, write_json, Ctx, Res};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub root: PathBuf,
    pub http: reqwest::Client,
    /// instance id -> process id of a running game
    pub running: Mutex<HashMap<String, u32>>,
    pub accounts_lock: tokio::sync::Mutex<()>,
    /// connection to the companion mod inside running games
    pub link: std::sync::Arc<crate::link::LinkHub>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Self {
        let root = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("AniLauncher"));
        Self::with_root(root)
    }

    pub fn with_root(root: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&root);
        Self {
            root,
            http: crate::net::client(),
            running: Mutex::new(HashMap::new()),
            accounts_lock: tokio::sync::Mutex::new(()),
            link: std::sync::Arc::new(crate::link::LinkHub::default()),
        }
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }
    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }
    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }
    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }
    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }
    pub fn media(&self) -> PathBuf {
        self.root.join("media")
    }
    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn settings(&self) -> Value {
        read_json(&self.settings_path()).unwrap_or_else(|| serde_json::json!({}))
    }

    /// Azure application (client) ID: baked in at build time, or set in settings.
    pub fn client_id(&self) -> Option<String> {
        if let Some(id) = self.settings().get("clientId").and_then(|v| v.as_str()) {
            if !id.trim().is_empty() {
                return Some(id.trim().to_string());
            }
        }
        option_env!("ANILAUNCHER_CLIENT_ID")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    pub fn memory_mb(&self) -> u32 {
        self.settings()
            .get("memoryMb")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(4096)
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Value {
    let mut s = state.settings();
    if let Some(obj) = s.as_object_mut() {
        obj.insert("dataDir".into(), Value::String(state.root.to_string_lossy().into()));
        obj.insert(
            "builtInClientId".into(),
            Value::Bool(option_env!("ANILAUNCHER_CLIENT_ID").map(|s| !s.is_empty()).unwrap_or(false)),
        );
    }
    s
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Value) -> Res<()> {
    let mut s = settings;
    if let Some(obj) = s.as_object_mut() {
        obj.remove("dataDir");
        obj.remove("builtInClientId");
    }
    write_json(&state.settings_path(), &s)
}

/// Copies a picked file (avatar, art, font) into the launcher's media folder.
#[tauri::command]
pub fn import_media(state: State<'_, AppState>, path: String) -> Res<String> {
    let src = PathBuf::from(&path);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_lowercase();
    let allowed = ["png", "jpg", "jpeg", "webp", "gif", "mp4", "webm", "ttf", "otf", "woff", "woff2"];
    if !allowed.contains(&ext.as_str()) {
        return Err("Этот тип файла не подходит".into());
    }
    std::fs::create_dir_all(state.media()).ctx("Не удалось создать папку")?;
    let dest = state
        .media()
        .join(format!("{}.{ext}", uuid::Uuid::new_v4().simple()));
    std::fs::copy(&src, &dest).ctx("Не удалось скопировать файл")?;
    Ok(dest.to_string_lossy().into())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Res<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Неверная ссылка".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).ctx("Не удалось открыть ссылку")
}

/// Theme export/import: plain text files the user picked in a dialog.
#[tauri::command]
pub fn read_text_file(path: String) -> Res<String> {
    let meta = std::fs::metadata(&path).ctx("Файл не найден")?;
    if meta.len() > 1_000_000 {
        return Err("Файл слишком большой".into());
    }
    std::fs::read_to_string(&path).ctx("Не удалось прочитать файл")
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Res<()> {
    let lower = path.to_lowercase();
    if !(lower.ends_with(".json") || lower.ends_with(".anitheme")) {
        return Err("Можно сохранять только файлы темы".into());
    }
    std::fs::write(&path, content).ctx("Не удалось сохранить файл")
}

/// Total memory of this computer in MB (for the memory slider and advice).
#[tauri::command]
pub fn system_memory() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() / 1024 / 1024
}

/// Opens a file from the launcher folder (crash report, log) in its default app.
#[tauri::command]
pub fn open_file(app: AppHandle, state: State<'_, AppState>, path: String) -> Res<()> {
    let p = std::fs::canonicalize(&path).ctx("Файл не найден")?;
    let root = std::fs::canonicalize(&state.root).ctx("Папка лаунчера не найдена")?;
    if !p.starts_with(&root) {
        return Err("Этот файл не из лаунчера".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(p.to_string_lossy(), None::<&str>)
        .ctx("Не удалось открыть файл")
}
