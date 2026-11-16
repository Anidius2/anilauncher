//! Skins: account skin and capes (Minecraft services), local collection, NameMC window.

use crate::auth;
use crate::state::AppState;
use crate::util::{now, read_json, write_json, Ctx, Res};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

const PROFILE: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedSkin {
    id: String,
    name: String,
    file: String,
    /// "classic" or "slim"
    variant: String,
    added: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct Library {
    skins: Vec<SavedSkin>,
}

fn skins_dir(state: &AppState) -> PathBuf {
    state.media().join("skins")
}

fn library_path(state: &AppState) -> PathBuf {
    skins_dir(state).join("library.json")
}

fn load_lib(state: &AppState) -> Library {
    read_json(&library_path(state)).unwrap_or_default()
}

fn save_lib(state: &AppState, lib: &Library) -> Res<()> {
    write_json(&library_path(state), lib)
}

/// Width and height from a PNG header, without decoding the image.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[1..4] != b"PNG" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

fn check_skin(bytes: &[u8]) -> Res<()> {
    match png_size(bytes) {
        Some((64, 64)) | Some((64, 32)) => Ok(()),
        Some((w, h)) => Err(format!("Скин должен быть 64×64 или 64×32, а этот {w}×{h}")),
        None => Err("Скин должен быть картинкой PNG".into()),
    }
}

fn add_to_library(state: &AppState, bytes: &[u8], name: &str, variant: &str) -> Res<SavedSkin> {
    check_skin(bytes)?;
    let dir = skins_dir(state);
    std::fs::create_dir_all(&dir).ctx("Не удалось создать папку скинов")?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let path = dir.join(format!("{id}.png"));
    std::fs::write(&path, bytes).ctx("Не удалось сохранить скин")?;
    let skin = SavedSkin {
        id,
        name: name.chars().take(40).collect(),
        file: path.to_string_lossy().into(),
        variant: if variant == "slim" { "slim".into() } else { "classic".into() },
        added: now(),
    };
    let mut lib = load_lib(state);
    lib.skins.insert(0, skin.clone());
    save_lib(state, &lib)?;
    Ok(skin)
}

#[tauri::command(async)]
pub fn list_skins(state: State<'_, AppState>) -> Vec<SavedSkin> {
    load_lib(&state)
        .skins
        .into_iter()
        .filter(|s| Path::new(&s.file).exists())
        .collect()
}

#[tauri::command(async)]
pub fn import_skin(state: State<'_, AppState>, path: String, name: String, variant: String) -> Res<SavedSkin> {
    let bytes = std::fs::read(&path).ctx("Не удалось открыть файл")?;
    add_to_library(&state, &bytes, &name, &variant)
}

/// Saves a skin drawn in the editor (PNG as base64). Replaces `id` when given.
#[tauri::command(async)]
pub fn save_skin_data(state: State<'_, AppState>, id: Option<String>, name: String, variant: String, data: String) -> Res<SavedSkin> {
    let raw = data.split_once(',').map(|(_, b)| b).unwrap_or(&data);
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw).ctx("Неверные данные скина")?;
    check_skin(&bytes)?;
    if let Some(id) = id {
        let mut lib = load_lib(&state);
        if let Some(s) = lib.skins.iter_mut().find(|s| s.id == id) {
            std::fs::write(&s.file, &bytes).ctx("Не удалось сохранить скин")?;
            s.name = name.chars().take(40).collect();
            s.variant = if variant == "slim" { "slim".into() } else { "classic".into() };
            let out = s.clone();
            save_lib(&state, &lib)?;
            return Ok(out);
        }
    }
    add_to_library(&state, &bytes, &name, &variant)
}

#[tauri::command(async)]
pub fn update_skin(state: State<'_, AppState>, id: String, name: String, variant: String) -> Res<()> {
    let mut lib = load_lib(&state);
    if let Some(s) = lib.skins.iter_mut().find(|s| s.id == id) {
        s.name = name.chars().take(40).collect();
        s.variant = if variant == "slim" { "slim".into() } else { "classic".into() };
    }
    save_lib(&state, &lib)
}

#[tauri::command(async)]
pub fn delete_skin(state: State<'_, AppState>, id: String) -> Res<()> {
    let mut lib = load_lib(&state);
    if let Some(s) = lib.skins.iter().find(|s| s.id == id) {
        let _ = std::fs::remove_file(&s.file);
    }
    lib.skins.retain(|s| s.id != id);
    save_lib(&state, &lib)
}

/// Image as a data: URL, so the 3D viewer can use it without cross-origin trouble.
#[tauri::command]
pub async fn texture_data(state: State<'_, AppState>, source: String) -> Res<String> {
    let bytes = if source.starts_with("https://textures.minecraft.net/") || source.starts_with("http://textures.minecraft.net/") {
        state
            .http
            .get(source.replace("http://", "https://"))
            .send()
            .await
            .ctx("Нет соединения")?
            .bytes()
            .await
            .ctx("Не удалось скачать текстуру")?
            .to_vec()
    } else {
        let p = std::fs::canonicalize(&source).ctx("Файл не найден")?;
        let root = std::fs::canonicalize(&state.root).ctx("Папка лаунчера не найдена")?;
        if !p.starts_with(&root) {
            return Err("Этот файл не из лаунчера".into());
        }
        std::fs::read(&p).ctx("Не удалось открыть файл")?
    };
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    id: String,
    alias: String,
    url: String,
    active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinProfile {
    skin_url: Option<String>,
    variant: String,
    capes: Vec<Cape>,
}

fn parse_profile(p: &Value) -> SkinProfile {
    let skin = p
        .get("skins")
        .and_then(|s| s.as_array())
        .and_then(|a| a.iter().find(|s| s.get("state").and_then(|x| x.as_str()) == Some("ACTIVE")));
    SkinProfile {
        skin_url: skin
            .and_then(|s| s.get("url"))
            .and_then(|u| u.as_str())
            .map(|u| u.replace("http://", "https://")),
        variant: skin
            .and_then(|s| s.get("variant"))
            .and_then(|v| v.as_str())
            .unwrap_or("CLASSIC")
            .to_lowercase(),
        capes: p
            .get("capes")
            .and_then(|c| c.as_array())
            .map(|a| {
                a.iter()
                    .map(|c| Cape {
                        id: c.get("id").and_then(|x| x.as_str()).unwrap_or("").into(),
                        alias: c.get("alias").and_then(|x| x.as_str()).unwrap_or("").into(),
                        url: c
                            .get("url")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .replace("http://", "https://"),
                        active: c.get("state").and_then(|x| x.as_str()) == Some("ACTIVE"),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

async fn token_for(state: &AppState) -> Res<String> {
    let file = auth::load(state);
    let active = file.active.clone().ok_or("Сначала войди в аккаунт")?;
    let acc = auth::ensure_fresh(state, &active).await?;
    if acc.kind != "msa" {
        return Err("Скин и плащи меняются только на лицензионном аккаунте".into());
    }
    acc.mc_token.ok_or_else(|| "Нужно войти заново".into())
}

async fn check(resp: reqwest::Response) -> Res<SkinProfile> {
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err("Слишком часто — подожди минуту и попробуй снова".into());
    }
    if !status.is_success() {
        return Err(format!("Сервер Minecraft ответил {status}"));
    }
    let v: Value = resp.json().await.ctx("Неверный ответ")?;
    Ok(parse_profile(&v))
}

#[tauri::command]
pub async fn skin_profile(state: State<'_, AppState>) -> Res<SkinProfile> {
    let token = token_for(&state).await?;
    check(state.http.get(PROFILE).bearer_auth(token).send().await.ctx("Нет соединения")?).await
}

/// Puts a skin from the collection on the licensed account.
#[tauri::command]
pub async fn upload_skin(state: State<'_, AppState>, id: String) -> Res<SkinProfile> {
    let lib = load_lib(&state);
    let skin = lib.skins.iter().find(|s| s.id == id).ok_or("Скин не найден")?;
    let bytes = std::fs::read(&skin.file).ctx("Не удалось открыть скин")?;
    let token = token_for(&state).await?;
    let form = reqwest::multipart::Form::new()
        .text("variant", skin.variant.clone())
        .part(
            "file",
            reqwest::multipart::Part::bytes(bytes)
                .file_name("skin.png")
                .mime_str("image/png")
                .ctx("Ошибка файла")?,
        );
    check(
        state
            .http
            .post(format!("{PROFILE}/skins"))
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .ctx("Нет соединения")?,
    )
    .await
}

#[tauri::command]
pub async fn set_cape(state: State<'_, AppState>, cape_id: Option<String>) -> Res<SkinProfile> {
    let token = token_for(&state).await?;
    let req = match cape_id {
        Some(id) => state
            .http
            .put(format!("{PROFILE}/capes/active"))
            .bearer_auth(token)
            .json(&json!({"capeId": id})),
        None => state.http.delete(format!("{PROFILE}/capes/active")).bearer_auth(token),
    };
    check(req.send().await.ctx("Нет соединения")?).await
}

/// Opens NameMC in its own window; skins downloaded there land in the collection.
#[tauri::command]
pub async fn open_namemc<R: Runtime>(app: AppHandle<R>) -> Res<()> {
    if let Some(w) = app.get_webview_window("namemc") {
        let _ = w.set_focus();
        return Ok(());
    }
    let incoming = app.state::<AppState>().media().join("skins").join("incoming");
    std::fs::create_dir_all(&incoming).ctx("Не удалось создать папку")?;
    let handle = app.clone();
    tauri::WebviewWindowBuilder::new(
        &app,
        "namemc",
        tauri::WebviewUrl::External("https://namemc.com/minecraft-skins".parse().ctx("Неверная ссылка")?),
    )
    .title("NameMC — AniLauncher")
    .inner_size(1180.0, 820.0)
    .on_download(move |_webview, event| {
        match event {
            tauri::webview::DownloadEvent::Requested { destination, .. } => {
                *destination = incoming.join(format!("{}.png", uuid::Uuid::new_v4().simple()));
            }
            tauri::webview::DownloadEvent::Finished { path, success, url } => {
                if success {
                    if let Some(p) = path {
                        let st = handle.state::<AppState>();
                        let name = url
                            .path_segments()
                            .and_then(|mut s| s.next_back().map(|x| x.trim_end_matches(".png").to_string()))
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| "NameMC".into());
                        let result = std::fs::read(&p)
                            .ctx("Не удалось открыть файл")
                            .and_then(|b| add_to_library(&st, &b, &format!("NameMC {}", &name[..name.len().min(8)]), "classic"));
                        let _ = std::fs::remove_file(&p);
                        let _ = match result {
                            Ok(s) => handle.emit("skin-added", s),
                            Err(e) => handle.emit("skin-error", e),
                        };
                    }
                }
            }
            _ => {}
        }
        true
    })
    .build()
    .ctx("Не удалось открыть NameMC")?;
    Ok(())
}

/// Opens any PNG the user picked (editor import) as a data: URL.
#[tauri::command(async)]
pub fn read_png(path: String) -> Res<String> {
    if !path.to_lowercase().ends_with(".png") {
        return Err("Нужен файл PNG".into());
    }
    let bytes = std::fs::read(&path).ctx("Не удалось открыть файл")?;
    check_skin(&bytes)?;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Saves the editor picture to a PNG file the user picked (editor export).
#[tauri::command(async)]
pub fn write_png(path: String, data: String) -> Res<()> {
    if !path.to_lowercase().ends_with(".png") {
        return Err("Сохранять можно только PNG".into());
    }
    let raw = data.split_once(',').map(|(_, b)| b).unwrap_or(&data);
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw).ctx("Неверные данные")?;
    check_skin(&bytes)?;
    std::fs::write(&path, bytes).ctx("Не удалось сохранить файл")
}
