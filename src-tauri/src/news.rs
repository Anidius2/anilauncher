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
    let _ = write_json(&cache, &feed);
    Ok(feed)
}
