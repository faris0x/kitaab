// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::api::QuranApiClient;

pub const AYATUL_KURSI_RECITERS: [(&str, &str); 5] = [
    (
        "Abu Bakr Al-Shatri",
        "https://raw.githubusercontent.com/fadsec-lab/quran-audios/main/abu-bakr-al-shatri/abu-bakr-al-shatri-ayatul-kursi.mp3",
    ),
    (
        "Mishary Rashid Alafasy",
        "https://raw.githubusercontent.com/fadsec-lab/quran-audios/main/al-afasy/al-afasy-ayatul-kursi.mp3",
    ),
    (
        "Muhammad Al Luhaidan",
        "https://raw.githubusercontent.com/fadsec-lab/quran-audios/main/muhammad_al_luhaidan/muhammad-al-luhaidan-ayatul-kursi.MP3",
    ),
    (
        "Nasser Al-Qatami",
        "https://raw.githubusercontent.com/fadsec-lab/quran-audios/main/nasser-al-qatami/nasser-al-qatami-ayatul-kursi.mp3",
    ),
    (
        "Yasser Al-Dossari",
        "https://raw.githubusercontent.com/fadsec-lab/quran-audios/main/yasser-al-dossari/yasser-al-dossari-ayatul-kursi.mp3",
    ),
];

pub fn sanitize_reciter(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_')
        .collect::<String>()
        .trim_end()
        .replace(' ', "_")
}

pub fn surah_audio_path(cache_dir: &Path, surah: u32, reciter: &str) -> PathBuf {
    cache_dir.join(format!("surah_{surah}_reciter_{}.mp3", sanitize_reciter(reciter)))
}

pub fn ayatul_kursi_audio_path(cache_dir: &Path, reciter: &str) -> PathBuf {
    cache_dir.join(format!("ayatul_kursi_{}.mp3", sanitize_reciter(reciter)))
}

pub async fn download_audio(
    client: &QuranApiClient,
    url: &str,
    path: &Path,
    fallback_url: Option<&str>,
) -> Result<()> {
    let attempt = async |url: &str| -> Result<()> {
        let response = client
            .client()
            .get(url)
            .send()
            .await
            .with_context(|| format!("failed to download {url}"))?;
        let bytes = response.error_for_status()?.bytes().await?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    };
    match attempt(url).await {
        Ok(()) => Ok(()),
        Err(first) => {
            if let Some(fallback) = fallback_url {
                attempt(fallback)
                    .await
                    .with_context(|| format!("primary failed ({first}); fallback also failed"))
            } else {
                Err(first)
            }
        }
    }
}

pub struct PlaybackState {
    pub surah: u32,
    pub reciter: String,
    pub duration: Duration,
}

pub struct AudioManager {
    output: Option<rodio::OutputStream>,
    sink: Option<rodio::Sink>,
    pub current: Option<PlaybackState>,
    pub paused: bool,
}

impl AudioManager {
    pub fn new() -> Self {
        match rodio::OutputStreamBuilder::open_default_stream() {
            Ok(output) => Self {
                output: Some(output),
                sink: None,
                current: None,
                paused: false,
            },
            Err(_) => Self {
                output: None,
                sink: None,
                current: None,
                paused: false,
            },
        }
    }

    pub fn available(&self) -> bool {
        self.output.is_some()
    }

    pub fn play_file(
        &mut self,
        path: &Path,
        surah: u32,
        reciter: String,
        _is_ayatul_kursi: bool,
    ) -> Result<()> {
        let output = self.output.as_ref().context("audio output unavailable")?;
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
        let byte_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        let decoder = rodio::Decoder::builder()
            .with_data(BufReader::new(file))
            .with_byte_len(byte_len)
            .build()
            .with_context(|| format!("cannot decode {}", path.display()))?;
        use rodio::Source as _;
        let duration = decoder.total_duration().unwrap_or_default();
        let sink = rodio::Sink::connect_new(output.mixer());
        sink.append(decoder);
        sink.play();
        self.sink = Some(sink);
        self.current = Some(PlaybackState {
            surah,
            reciter,
            duration,
        });
        self.paused = false;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        self.sink = None;
        self.current = None;
        self.paused = false;
    }

    pub fn toggle_pause(&mut self) {
        let Some(sink) = &self.sink else {
            return;
        };
        if self.paused {
            sink.play();
            self.paused = false;
        } else {
            sink.pause();
            self.paused = true;
        }
    }

    pub fn has_playback(&self) -> bool {
        self.sink.as_ref().is_some_and(|sink| !sink.empty())
    }

    pub fn status_line(&self) -> Option<String> {
        let state = self.current.as_ref()?;
        let sink = self.sink.as_ref()?;
        if sink.empty() {
            return None;
        }
        let position = sink.get_pos();
        let (pm, ps) = (position.as_secs() / 60, position.as_secs() % 60);
        let (dm, ds) = (state.duration.as_secs() / 60, state.duration.as_secs() % 60);
        let icon = if self.paused { "⏸" } else { "⏵" };
        Some(format!(
            "{icon} {} {pm}:{ps:02} / {dm}:{ds:02}  (Space pause, s stop)",
            state.reciter
        ))
    }
}