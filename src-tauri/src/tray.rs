//! Icon next to the clock: the launcher can live there while the game runs or after its
//! window is closed. The menu is rebuilt by the interface whenever builds or the game change.

use crate::util::{Ctx, Res};
use serde::Deserialize;
use serde_json::json;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const TRAY_ID: &str = "main";

fn icon(playing: bool) -> Option<Image<'static>> {
    let bytes: &'static [u8] = if playing { include_bytes!("../icons/tray-playing.png") } else { include_bytes!("../icons/tray.png") };
    Image::from_bytes(bytes).ok()
}

/// Brings the window back from the tray (or from being minimized).
pub fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn create<R: Runtime>(app: &AppHandle<R>) -> Res<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Открыть AniLauncher", true, None::<&str>).ctx("Трей")?,
            &PredefinedMenuItem::separator(app).ctx("Трей")?,
            &MenuItem::with_id(app, "quit", "Выйти", true, None::<&str>).ctx("Трей")?,
        ],
    )
    .ctx("Трей")?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("AniLauncher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(tray.app_handle());
            }
        });
    if let Some(i) = icon(false) {
        builder = builder.icon(i);
    }
    builder.build(app).ctx("Не удалось создать значок в трее")?;
    Ok(())
}

fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "open" => show_window(app),
        "quit" => app.exit(0),
        other => {
            // play:<id>, stop:<id>, remote:<id>, streamer — handled by the interface
            if other.starts_with("remote:") {
                show_window(app);
            }
            let (action, target) = other.split_once(':').unwrap_or((other, ""));
            let _ = app.emit("tray", json!({"action": action, "id": target}));
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayBuild {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayState {
    builds: Vec<TrayBuild>,
    /// running builds: (id, name)
    running: Vec<TrayBuild>,
    /// running builds that have AniLauncher Link connected
    linked: Vec<String>,
    streamer: bool,
    lang: String,
}

/// Rebuilds the menu, icon and tooltip from what the interface knows.
#[tauri::command(async)]
pub fn tray_update<R: Runtime>(app: AppHandle<R>, state: TrayState) -> Res<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return Ok(()) };
    let ru = state.lang != "en";
    let tr = |a: &str, b: &str| if ru { a.to_string() } else { b.to_string() };
    let cut = |s: &str| if s.chars().count() > 36 { format!("{}…", s.chars().take(35).collect::<String>()) } else { s.to_string() };

    let menu = Menu::new(&app).ctx("Трей")?;
    menu.append(&MenuItem::with_id(&app, "open", tr("Открыть AniLauncher", "Open AniLauncher"), true, None::<&str>).ctx("Трей")?).ctx("Трей")?;
    menu.append(&PredefinedMenuItem::separator(&app).ctx("Трей")?).ctx("Трей")?;

    for r in &state.running {
        let sub = Submenu::new(&app, format!("▶ {}", cut(&r.name)), true).ctx("Трей")?;
        if state.linked.contains(&r.id) {
            sub.append(&MenuItem::with_id(&app, format!("remote:{}", r.id), tr("Пульт игры", "Game remote"), true, None::<&str>).ctx("Трей")?).ctx("Трей")?;
        }
        sub.append(&MenuItem::with_id(&app, format!("console:{}", r.id), tr("Консоль", "Console"), true, None::<&str>).ctx("Трей")?).ctx("Трей")?;
        sub.append(&MenuItem::with_id(&app, format!("stop:{}", r.id), tr("Остановить игру", "Stop the game"), true, None::<&str>).ctx("Трей")?).ctx("Трей")?;
        menu.append(&sub).ctx("Трей")?;
    }
    if !state.running.is_empty() {
        menu.append(&PredefinedMenuItem::separator(&app).ctx("Трей")?).ctx("Трей")?;
    }

    let idle: Vec<&TrayBuild> = state.builds.iter().filter(|b| !state.running.iter().any(|r| r.id == b.id)).take(5).collect();
    if !idle.is_empty() {
        let play = Submenu::new(&app, tr("Играть", "Play"), true).ctx("Трей")?;
        for b in idle {
            play.append(&MenuItem::with_id(&app, format!("play:{}", b.id), cut(&b.name), true, None::<&str>).ctx("Трей")?).ctx("Трей")?;
        }
        menu.append(&play).ctx("Трей")?;
    }
    menu.append(&CheckMenuItem::with_id(&app, "streamer", tr("Режим «Ютубер»", "YouTuber mode"), true, state.streamer, None::<&str>).ctx("Трей")?).ctx("Трей")?;
    menu.append(&PredefinedMenuItem::separator(&app).ctx("Трей")?).ctx("Трей")?;
    menu.append(&MenuItem::with_id(&app, "quit", tr("Выйти", "Quit"), true, None::<&str>).ctx("Трей")?).ctx("Трей")?;
    tray.set_menu(Some(menu)).ctx("Трей")?;

    let playing = !state.running.is_empty();
    let _ = tray.set_icon(icon(playing));
    let tip = match state.running.first() {
        Some(r) if !state.streamer => format!("AniLauncher — {}: {}", tr("играешь", "playing"), r.name),
        Some(_) => format!("AniLauncher — {}", tr("игра запущена", "game running")),
        None => "AniLauncher".to_string(),
    };
    let _ = tray.set_tooltip(Some(tip));
    Ok(())
}

/// Hides the window to the tray.
#[tauri::command(async)]
pub fn hide_to_tray<R: Runtime>(app: AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

/// Start with Windows (hidden in the tray).
#[tauri::command(async)]
pub fn set_autostart<R: Runtime>(app: AppHandle<R>, on: bool) -> Res<bool> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if on {
        al.enable().ctx("Не удалось включить автозапуск")?;
    } else {
        al.disable().ctx("Не удалось выключить автозапуск")?;
    }
    al.is_enabled().ctx("Не удалось проверить автозапуск")
}
