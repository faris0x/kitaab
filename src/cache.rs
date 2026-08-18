use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::models::{CachedSurah, SurahTranslation, TranslationDb};

pub const TOTAL_SURAHS: u32 = 114;

pub const MAIN_DB_URL: &str =
    "https://gist.githubusercontent.com/raz0229/ad8cb85f6a92c08f7acf50d245524a5f/raw";
pub const TR_DB_URL: &str =
    "https://raw.githubusercontent.com/saikothasan/quran-cloud/main/quran_tr.json";

#[derive(Debug, Default, Clone)]
pub struct QuranCache {
    path: PathBuf,
    pub data: HashMap<String, CachedSurah>,
}

impl QuranCache {
    pub fn load(path: &Path) -> Self {
        let data = match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => HashMap::new(),
        };
        Self {
            path: path.to_path_buf(),
            data,
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string(&self.data)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn missing_surahs(&self) -> Vec<u32> {
        let mut missing = Vec::new();
        for n in 1..=TOTAL_SURAHS {
            let valid = self.get(n).is_some_and(is_valid_surah);
            if !valid {
                missing.push(n);
            }
        }
        missing
    }

    pub fn get(&self, n: u32) -> Option<&CachedSurah> {
        self.data.get(&n.to_string())
    }

    pub fn insert(&mut self, n: u32, surah: CachedSurah) {
        self.data.insert(n.to_string(), surah);
    }

    pub fn surah_names(&self) -> Vec<String> {
        (1..=TOTAL_SURAHS)
            .map(|n| {
                self.get(n)
                    .map(|s| s.surah_name.clone())
                    .unwrap_or_else(|| format!("Surah {n}"))
            })
            .collect()
    }
}

fn is_valid_surah(surah: &CachedSurah) -> bool {
    !surah.surah_name.is_empty()
        && !surah.surah_name_arabic.is_empty()
        && surah.total_ayah > 0
        && !surah.arabic_uthmani.is_empty()
        && !surah.arabic_simple.is_empty()
        && !surah.english.is_empty()
}

#[derive(Debug, Clone)]
pub struct DatabaseFiles {
    pub main_db: PathBuf,
    pub tr_db: PathBuf,
}

impl DatabaseFiles {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            main_db: cache_dir.join("quran-translation-and-transliteration.json"),
            tr_db: cache_dir.join("quran_tr.json"),
        }
    }

    pub fn missing(&self) -> Vec<(&'static str, &PathBuf)> {
        let mut missing = Vec::new();
        for (url, path) in [(MAIN_DB_URL, &self.main_db), (TR_DB_URL, &self.tr_db)] {
            if !path.exists() {
                missing.push((url, path));
            }
        }
        missing
    }

    pub fn load_main(&self) -> Result<TranslationDb> {
        let contents = std::fs::read_to_string(&self.main_db)
            .with_context(|| format!("failed to read {}", self.main_db.display()))?;
        serde_json::from_str(&contents).with_context(|| "failed to parse main translation DB")
    }

    pub fn load_tr(&self) -> Result<Vec<SurahTranslation>> {
        let contents = std::fs::read_to_string(&self.tr_db)
            .with_context(|| format!("failed to read {}", self.tr_db.display()))?;
        serde_json::from_str(&contents).with_context(|| "failed to parse Turkish DB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_surah() -> CachedSurah {
        serde_json::from_str(
            r#"{
                "surahName": "Al-Faatiha",
                "surahNameArabic": "الفاتحة",
                "totalAyah": 7,
                "surahNo": 1,
                "audio": {},
                "english": ["a"],
                "arabic1": ["ب"],
                "arabic2": ["ج"]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn missing_surahs_reports_everything_when_empty() {
        let cache = QuranCache {
            path: PathBuf::new(),
            data: HashMap::new(),
        };
        assert_eq!(cache.missing_surahs().len(), 114);
    }

    #[test]
    fn missing_surahs_accepts_valid_entry() {
        let mut data = HashMap::new();
        data.insert("1".into(), sample_surah());
        let cache = QuranCache {
            path: PathBuf::new(),
            data,
        };
        let missing = cache.missing_surahs();
        assert!(!missing.contains(&1));
        assert_eq!(missing.len(), 113);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = std::env::temp_dir().join(format!("kitaab-cache-test-{}.json", std::process::id()));
        let mut cache = QuranCache::load(&path);
        cache.insert(1, sample_surah());
        cache.save().unwrap();
        let loaded = QuranCache::load(&path);
        assert!(loaded.get(1).is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_cache_file_resets_to_empty() {
        let path = std::env::temp_dir().join(format!("kitaab-cache-corrupt-{}.json", std::process::id()));
        std::fs::write(&path, "not json").unwrap();
        let cache = QuranCache::load(&path);
        assert!(cache.data.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn surah_names_fallback() {
        let mut data = HashMap::new();
        data.insert("1".into(), sample_surah());
        let cache = QuranCache {
            path: PathBuf::new(),
            data,
        };
        let names = cache.surah_names();
        assert_eq!(names[0], "Al-Faatiha");
        assert_eq!(names[1], "Surah 2");
    }
}