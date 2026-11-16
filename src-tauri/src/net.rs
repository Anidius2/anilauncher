use crate::util::{Ctx, Res};
use futures::stream::{self, StreamExt};
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("AniLauncher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("http client")
}

pub async fn get_json<T: serde::de::DeserializeOwned>(http: &reqwest::Client, url: &str) -> Res<T> {
    let resp = http.get(url).send().await.ctx("Нет соединения")?;
    if !resp.status().is_success() {
        return Err(format!("Сервер ответил {} ({url})", resp.status()));
    }
    resp.json::<T>().await.ctx("Не удалось прочитать ответ")
}

fn sha1_of(path: &PathBuf) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(Sha1::digest(&bytes)))
}

/// A file counts as present when it exists and its size matches (or no size is known).
fn present(d: &Download) -> bool {
    match std::fs::metadata(&d.path) {
        Ok(meta) => match d.size {
            Some(size) => meta.len() == size,
            None => meta.len() > 0,
        },
        Err(_) => false,
    }
}

async fn fetch_one(http: &reqwest::Client, d: &Download) -> Res<()> {
    if present(d) {
        return Ok(());
    }
    if let Some(parent) = d.path.parent() {
        tokio::fs::create_dir_all(parent).await.ctx("Не удалось создать папку")?;
    }
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(600 * attempt)).await;
        }
        let resp = match http.get(&d.url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = e.to_string();
                continue;
            }
        };
        if !resp.status().is_success() {
            last_err = format!("HTTP {}", resp.status());
            continue;
        }
        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                last_err = e.to_string();
                continue;
            }
        };
        if let Some(expected) = &d.sha1 {
            let actual = hex::encode(Sha1::digest(&bytes));
            if !actual.eq_ignore_ascii_case(expected) {
                last_err = "контрольная сумма не совпала".into();
                continue;
            }
        }
        let mut tmp_name = d.path.as_os_str().to_owned();
        tmp_name.push(".part");
        let tmp = PathBuf::from(tmp_name);
        tokio::fs::write(&tmp, &bytes).await.ctx("Не удалось записать файл")?;
        tokio::fs::rename(&tmp, &d.path).await.ctx("Не удалось сохранить файл")?;
        return Ok(());
    }
    Err(format!("Не удалось скачать {}: {last_err}", d.url))
}

/// Downloads everything in parallel, calling `progress(done, total)` as files finish.
pub async fn download_all<F>(http: &reqwest::Client, items: Vec<Download>, progress: F) -> Res<()>
where
    F: Fn(usize, usize) + Send + Sync + 'static,
{
    let total = items.len();
    let done = Arc::new(AtomicUsize::new(0));
    let progress = Arc::new(progress);
    progress(0, total);
    let results: Vec<Res<()>> = stream::iter(items)
        .map(|d| {
            let http = http.clone();
            let done = done.clone();
            let progress = progress.clone();
            async move {
                let r = fetch_one(&http, &d).await;
                let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                if n % 16 == 0 || n == total {
                    progress(n, total);
                }
                r
            }
        })
        .buffer_unordered(24)
        .collect()
        .await;
    for r in results {
        r?;
    }
    Ok(())
}

/// Checks an already-downloaded file against its hash (used for small, critical files).
#[allow(dead_code)]
pub fn verify(path: &PathBuf, sha1: &str) -> bool {
    sha1_of(path).map(|s| s.eq_ignore_ascii_case(sha1)).unwrap_or(false)
}
