//! Official Minecraft news and patch notes (with their art) from Mojang's launcher feed.

use crate::net::get_json;
use crate::state::AppState;
use crate::util::{read_json, write_json, Res};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

const BASE: &str = "https://launchercontent.mojang.com";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    id: String,
    title: String,
    tag: String,
    date: String,
    image: Option<String>,
    link: Option<String>,
    text: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    news: Vec<NewsItem>,
    patch_notes: Vec<NewsItem>,
    /// Full-size key art (local files), taken from the articles on minecraft.net
    #[serde(default)]
    art: Vec<String>,
}

fn abs(url: Option<&str>) -> Option<String> {
    let u = url?;
    if u.starts_with("http") {
        Some(u.replace("http://", "https://"))
    } else {
        Some(format!("{BASE}{}{u}", if u.starts_with('/') { "" } else { "/" }))
    }
}

fn s(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// News and patch notes; falls back to the last saved copy when offline.
#[tauri::command]
pub async fn get_feed(state: State<'_, AppState>) -> Res<Feed> {
    let cache = state.root.join("cache").join("feed.json");
    let news: Result<Value, String> = get_json(&state.http, &format!("{BASE}/v2/news.json")).await;
    let notes: Result<Value, String> = get_json(&state.http, &format!("{BASE}/v2/javaPatchNotes.json")).await;
    if news.is_err() && notes.is_err() {
        return read_json(&cache).ok_or_else(|| "Не удалось загрузить новости".to_string());
    }
    let mut feed = Feed::default();
    // Key art: the main game first (entries for Java), then other big Minecraft art; no Realms,
    // Marketplace ads or spin-offs
    let mut art_links: Vec<String> = Vec::new();
    if let Ok(n) = &news {
        let entries: Vec<&Value> = n.get("entries").and_then(|e| e.as_array()).map(|a| a.iter().collect()).unwrap_or_default();
        let types = |e: &Value| -> Vec<String> {
            e.get("newsType").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
        };
        let side = |e: &Value| {
            let title = s(e, "title").to_lowercase();
            ["realms", "marketplace", "dungeons", "legends", "domino", "add-on", "sound effect", "pre-order", "cape"].iter().any(|w| title.contains(w))
                || s(e, "category").contains("Dungeons")
        };
        let java: Vec<&&Value> = entries.iter().filter(|e| types(e).iter().any(|t| t == "Java") && !side(e)).collect();
        let other: Vec<&&Value> = entries.iter().filter(|e| !types(e).iter().any(|t| t == "Java") && !side(e)).collect();
        for e in java.into_iter().chain(other.into_iter().take(10)) {
            if let Some(l) = e.get("readMoreLink").and_then(|l| l.as_str()) {
                if !art_links.iter().any(|x| x == l) {
                    art_links.push(l.to_string());
                }
            }
            if art_links.len() >= 20 {
                break;
            }
        }
    }
    if let Ok(n) = news {
        for e in n.get("entries").and_then(|e| e.as_array()).into_iter().flatten() {
            let for_java = e
                .get("newsType")
                .and_then(|t| t.as_array())
                .map(|a| a.iter().any(|x| x.as_str() == Some("Java")))
                .unwrap_or(true);
            if !for_java {
                continue;
            }
            feed.news.push(NewsItem {
                id: s(e, "id"),
                title: s(e, "title"),
                tag: s(e, "tag"),
                date: s(e, "date"),
                image: abs(
                    e.pointer("/newsPageImage/url")
                        .or_else(|| e.pointer("/playPageImage/url"))
                        .and_then(|u| u.as_str()),
                ),
                link: e.get("readMoreLink").and_then(|l| l.as_str()).map(String::from),
                text: s(e, "text"),
            });
            if feed.news.len() >= 24 {
                break;
            }
        }
    }
    if let Ok(n) = notes {
        for e in n.get("entries").and_then(|e| e.as_array()).into_iter().flatten() {
            feed.patch_notes.push(NewsItem {
                id: s(e, "id"),
                title: s(e, "title"),
                tag: s(e, "type"),
                date: s(e, "date"),
                image: abs(e.pointer("/image/url").and_then(|u| u.as_str())),
                link: None,
                text: s(e, "shortText"),
            });
            if feed.patch_notes.len() >= 60 {
                break;
            }
        }
    }
    feed.art = key_art(&state, &art_links).await;
    spawn_wallpapers(&state);
    let _ = write_json(&cache, &feed);
    Ok(feed)
}

/// The launcher feed only has small pictures (772×350). Each article on minecraft.net names its
/// full-size header in og:image; we look it up once per article and remember the answer.
async fn key_art(state: &AppState, links: &[String]) -> Vec<String> {
    use std::collections::HashMap;
    let dir = state.root.join("cache").join("hero");
    let index = dir.join("index.json");
    // article link -> saved file name (None: the article has no usable picture)
    let mut known: HashMap<String, Option<String>> = read_json(&index).unwrap_or_default();
    let todo: Vec<String> = links
        .iter()
        .filter(|l| match known.get(*l) {
            Some(Some(f)) => !dir.join(f).exists(),
            Some(None) => false,
            None => true,
        })
        .cloned()
        .collect();
    if !todo.is_empty() {
        let _ = std::fs::create_dir_all(&dir);
        let jobs = todo.into_iter().map(|link| {
            let http = state.http.clone();
            let dir = dir.clone();
            async move {
                // Mojang's site doesn't let other pages show its pictures directly, so we keep a copy
                let job = async {
                    let page = link.split('?').next().unwrap_or(&link).to_string();
                    let html = http.get(&page).send().await.ok()?.text().await.ok()?;
                    let url = og_image(&html)?;
                    let resp = http.get(&url).send().await.ok()?;
                    if !resp.status().is_success() {
                        return None;
                    }
                    let bytes = resp.bytes().await.ok()?;
                    // only wide, sharp pictures make a good background
                    let size = imagesize::blob_size(&bytes).ok()?;
                    if size.width < 1100 || (size.width as f32) < size.height as f32 * 1.9 {
                        return None;
                    }
                    let ext = url.rsplit('.').next().unwrap_or("jpg").split('?').next().unwrap_or("jpg").to_lowercase();
                    let name = format!("{:016x}.{ext}", fxhash(&url));
                    std::fs::write(dir.join(&name), &bytes).ok()?;
                    Some(name)
                };
                let got = tokio::time::timeout(std::time::Duration::from_secs(20), job).await.ok().flatten();
                (link, got)
            }
        });
        for (link, file) in futures::future::join_all(jobs).await {
            known.insert(link, file);
        }
        let _ = write_json(&index, &known);
    }
    let mut out: Vec<String> = Vec::new();
    for l in links {
        if let Some(Some(file)) = known.get(l) {
            let path = dir.join(file).to_string_lossy().into_owned();
            if dir.join(file).exists() && !out.contains(&path) {
                out.push(path);
            }
        }
    }
    out
}

/// Folder with Mojang's official wallpapers (big PNGs, up to 2560×1440) for the in-game menus.
pub fn wallpaper_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join("cache").join("wallpapers")
}

/// The newest drops' wallpapers. Each archive is ~16 MB, so it runs in the background, once a week,
/// and keeps only the one picture we need from each.
pub fn spawn_wallpapers(state: &AppState) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static BUSY: AtomicBool = AtomicBool::new(false);
    let dir = wallpaper_dir(&state.root);
    let stamp = dir.join("checked");
    let fresh = std::fs::metadata(&stamp)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < std::time::Duration::from_secs(7 * 24 * 3600));
    if fresh || BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let http = state.http.clone();
    tauri::async_runtime::spawn(async move {
        let ok = fetch_wallpapers(&http, &dir).await.is_some();
        if ok {
            let _ = std::fs::write(&stamp, b"");
        }
        BUSY.store(false, Ordering::SeqCst);
    });
}

const WALLPAPERS: usize = 6;

async fn fetch_wallpapers(http: &reqwest::Client, dir: &std::path::Path) -> Option<()> {
    let html = http.get("https://www.minecraft.net/en-us/collectibles").send().await.ok()?.error_for_status().ok()?.text().await.ok()?;
    let re = regex::Regex::new(r"/content/dam/minecraftnet/games/minecraft/software/(wallpapers_[A-Za-z0-9_\-()]+\.zip)").ok()?;
    let mut zips: Vec<String> = Vec::new();
    for c in re.captures_iter(&html) {
        let name = c[1].to_string();
        // the game-update archives; the edition/bundle ones are logo packs
        if (name.contains("_drop") || name.contains("_update")) && !zips.contains(&name) {
            zips.push(name);
        }
    }
    zips.truncate(WALLPAPERS);
    if zips.is_empty() {
        return None;
    }
    std::fs::create_dir_all(dir).ok()?;
    let wanted: Vec<String> = zips.iter().map(|z| z.trim_end_matches(".zip").to_string() + ".png").collect();
    for (zip, png) in zips.iter().zip(&wanted) {
        let out = dir.join(png);
        if out.exists() {
            continue;
        }
        let url = format!("https://www.minecraft.net/content/dam/minecraftnet/games/minecraft/software/{zip}");
        let job = async {
            let bytes = http.get(&url).send().await.ok()?.error_for_status().ok()?.bytes().await.ok()?;
            let picked = pick_wallpaper(&bytes)?;
            let tmp = out.with_extension("part");
            std::fs::write(&tmp, picked).ok()?;
            std::fs::rename(&tmp, &out).ok()
        };
        let _ = tokio::time::timeout(std::time::Duration::from_secs(180), job).await;
    }
    // drop wallpapers of drops that left the newest list
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if n.ends_with(".png") && !wanted.contains(&n) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    Some(())
}

/// From a wallpaper archive: the widescreen PNG closest to 2560×1440 (never smaller than 1920).
fn pick_wallpaper(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let re = regex::Regex::new(r"(\d{3,4})x(\d{3,4})\.png$").ok()?;
    let mut best: Option<(usize, u32)> = None;
    for i in 0..zip.len() {
        let name = zip.by_index(i).ok()?.name().to_lowercase();
        let Some(c) = re.captures(&name) else { continue };
        let (w, h): (u32, u32) = (c[1].parse().ok()?, c[2].parse().ok()?);
        if w < 1920 || w * 9 < h * 15 || w > 2560 {
            continue; // phone, square and odd sizes
        }
        if best.map_or(true, |(_, bw)| w > bw) {
            best = Some((i, w));
        }
    }
    let mut f = zip.by_index(best?.0).ok()?;
    let mut out = Vec::new();
    f.read_to_end(&mut out).ok()?;
    let size = imagesize::blob_size(&out).ok()?;
    (size.width >= 1920).then_some(out)
}

fn fxhash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

fn og_image(html: &str) -> Option<String> {
    let at = html.find("og:image")?;
    let rest = &html[at..html.len().min(at + 600)];
    let start = rest.find("content=\"")? + 9;
    let url = &rest[start..start + rest[start..].find('"')?];
    let url = if url.starts_with('/') { format!("https://www.minecraft.net{url}") } else { url.replace("http://", "https://") };
    // only real pictures from Mojang's own sites
    let ok = (url.starts_with("https://www.minecraft.net/") || url.starts_with("https://launchercontent.mojang.com/"))
        && [".jpg", ".jpeg", ".png", ".webp"].iter().any(|e| url.to_lowercase().split('?').next().unwrap_or("").ends_with(e));
    ok.then_some(url)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore]
    async fn official_wallpapers() {
        let dir = std::path::PathBuf::from("/tmp/wallpapers-test");
        let http = reqwest::Client::builder().user_agent("AniLauncher").build().unwrap();
        super::fetch_wallpapers(&http, &dir).await.expect("wallpapers");
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let s = imagesize::size(e.path()).unwrap();
            println!("{} {}x{}", e.file_name().to_string_lossy(), s.width, s.height);
            assert!(s.width >= 1920);
            n += 1;
        }
        assert!(n >= 3);
    }

    #[tokio::test]
    #[ignore]
    async fn real_feed_has_big_art() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-feedtest");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(crate::state::AppState::with_root(root));
        let feed = super::get_feed(app.state()).await.unwrap();
        let heroes = feed.art.clone();
        println!("news {} heroes {}", feed.news.len(), heroes.len());
        for h in &heroes { println!("{h} {}", std::fs::metadata(h).unwrap().len()); }
        assert!(!heroes.is_empty());
    }

    #[test]
    fn finds_og_image() {
        let h = r#"<meta property="og:image" content="https://www.minecraft.net/content/dam/a/key-art/X_1170x500.jpg"/>"#;
        assert_eq!(super::og_image(h).as_deref(), Some("https://www.minecraft.net/content/dam/a/key-art/X_1170x500.jpg"));
        assert_eq!(super::og_image(r#"<meta property="og:image" content="https://evil.example/x.jpg"/>"#), None);
    }
}
