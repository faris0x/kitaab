use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::{mpsc, Semaphore};

use crate::models::CachedSurah;

pub const BASE_URL: &str = "https://quranapi.pages.dev/api/";
pub const USER_AGENT: &str = "kitaab/0.1";
pub const MAX_CONCURRENT: usize = 5;
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub enum DownloadProgress {
    Started(u32),
    Finished(u32),
    Failed(u32),
    Done,
}

#[derive(Clone)]
pub struct QuranApiClient {
    client: reqwest::Client,
}

impl QuranApiClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { client })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn fetch_surah(&self, n: u32) -> Result<CachedSurah> {
        let url = format!("{BASE_URL}{n}.json");
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to fetch surah {n}"))?;
        let response = response
            .error_for_status()
            .with_context(|| format!("bad response for surah {n}"))?;
        response
            .json()
            .await
            .with_context(|| format!("failed to parse surah {n}"))
    }

    pub async fn download_missing(
        &self,
        missing: Vec<u32>,
        tx: mpsc::Sender<DownloadProgress>,
    ) -> HashMap<u32, CachedSurah> {
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
        let mut downloaded: HashMap<u32, CachedSurah> = HashMap::new();

        let mut handles = Vec::new();
        for n in missing.clone() {
            let semaphore = semaphore.clone();
            let tx = tx.clone();
            let client = self.client.clone();
            handles.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await;
                let _ = tx.send(DownloadProgress::Started(n)).await;
                let url = format!("{BASE_URL}{n}.json");
                let result = async {
                    let response = client
                        .get(&url)
                        .timeout(Duration::from_secs(15))
                        .send()
                        .await?;
                    let surah: CachedSurah = response.error_for_status()?.json().await?;
                    Ok::<_, anyhow::Error>(surah)
                }
                .await;
                match result {
                    Ok(surah) => {
                        let _ = tx.send(DownloadProgress::Finished(n)).await;
                        (n, Some(surah))
                    }
                    Err(_) => {
                        let _ = tx.send(DownloadProgress::Failed(n)).await;
                        (n, None)
                    }
                }
            }));
        }

        for handle in handles {
            if let Ok((n, Some(surah))) = handle.await {
                downloaded.insert(n, surah);
            }
        }

        let failed: Vec<u32> = missing
            .iter()
            .copied()
            .filter(|n| !downloaded.contains_key(n))
            .collect();
        for n in failed {
            match self.fetch_surah(n).await {
                Ok(surah) => {
                    downloaded.insert(n, surah);
                    let _ = tx.send(DownloadProgress::Finished(n)).await;
                }
                Err(_) => {
                    let _ = tx.send(DownloadProgress::Failed(n)).await;
                }
            }
        }

        let _ = tx.send(DownloadProgress::Done).await;
        downloaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_builds_surah_paths() {
        assert_eq!(
            format!("{BASE_URL}{}.json", 114),
            "https://quranapi.pages.dev/api/114.json"
        );
    }
}