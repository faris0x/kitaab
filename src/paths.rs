// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;

use anyhow::Result;
use directories::ProjectDirs;

pub struct Paths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub audio_cache_dir: PathBuf,
    pub settings_file: PathBuf,
    pub data_file: PathBuf,
    pub subtitles_dir: PathBuf,
    #[allow(dead_code)]
    pub documents_dir: PathBuf,
}

impl Paths {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", "kitaab")
            .ok_or_else(|| anyhow::anyhow!("could not determine platform data directories"))?;
        let config_dir = dirs.config_dir().to_path_buf();
        let cache_dir = dirs.cache_dir().to_path_buf();
        let audio_cache_dir = cache_dir.join("audio_cache");
        let settings_file = config_dir.join("kitaab-settings.json");
        let data_file = cache_dir.join("quran_data.json");
        let documents_dir = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("could not determine documents directory"))?
            .document_dir()
            .map(PathBuf::from)
            .unwrap_or_else(|| config_dir.join("documents"));
        let subtitles_dir = documents_dir.join("kitaab Subtitles");
        Ok(Self {
            config_dir,
            cache_dir,
            audio_cache_dir,
            settings_file,
            data_file,
            subtitles_dir,
            documents_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.audio_cache_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_resolve_to_home_dirs() {
        let paths = Paths::new().unwrap();
        assert!(paths.config_dir.ends_with("kitaab"));
        assert!(paths.cache_dir.ends_with("kitaab"));
        assert!(paths.settings_file.ends_with("kitaab-settings.json"));
        assert!(paths.data_file.ends_with("quran_data.json"));
        assert!(paths.audio_cache_dir.ends_with("audio_cache"));
    }

    #[test]
    fn subtitles_dir_nests_under_documents() {
        let paths = Paths::new().unwrap();
        assert!(paths.subtitles_dir.ends_with("kitaab Subtitles"));
    }
}