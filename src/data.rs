// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use anyhow::Result;

use crate::arabic::fix_arabic_text;
use crate::cache::{DatabaseFiles, QuranCache};
use crate::models::{Chapter, ReciterAudio, SurahTranslation};

pub const LUHAIDAN_KEY: &str = "luhaidan";
pub const LUHAIDAN_NAME: &str = "Muhammad Al Luhaidan";
pub const LUHAIDAN_URL: &str = "https://download.quranicaudio.com/quran/muhammad_alhaidan/{NNN}.mp3";
pub const LUHAIDAN_FALLBACK_URL: &str =
    "https://raw.githubusercontent.com/fadsec-lab/quran-audios/main/muhammad_al_luhaidan/muhammad-al-luhaidan-{NNN}.mp3";
pub const LUHAIDAN_FALLBACK_SURAHS: [u32; 4] = [2, 6, 25, 112];

#[derive(Debug, Clone)]
pub struct SurahInfo {
    pub surah_name: String,
    pub surah_name_ar: String,
    pub translation: String,
    pub revelation_type: String,
    pub total_verses: u32,
    pub description: Option<String>,
    pub surah_number: u32,
    #[allow(dead_code)]
    pub audio: HashMap<String, ReciterAudio>,
}

#[derive(Debug, Clone)]
pub struct Ayah {
    pub number: u32,
    pub content: String,
    pub transliteration: String,
    pub text: String,
    pub translation_tr: String,
}

pub struct DataHandler {
    pub cache: QuranCache,
    main_db: HashMap<String, Chapter>,
    tr_db: Vec<SurahTranslation>,
    pub arabic_reversed: bool,
}

impl DataHandler {
    pub fn load(cache: QuranCache, db_files: &DatabaseFiles) -> Result<Self> {
        let main_db = db_files.load_main()?.chapters;
        let tr_db = db_files.load_tr()?;
        Ok(Self {
            cache,
            main_db,
            tr_db,
            arabic_reversed: false,
        })
    }

    pub fn get_surah_info(&self, n: u32) -> Option<SurahInfo> {
        let cached = self.cache.get(n)?;
        let chapter = self.main_db.get(&n.to_string());
        Some(SurahInfo {
            surah_name: cached.surah_name.clone(),
            surah_name_ar: cached.surah_name_arabic.clone(),
            translation: chapter
                .map(|c| c.translation.clone())
                .or_else(|| cached.surah_name_translation.clone())
                .unwrap_or_default(),
            revelation_type: chapter
                .map(|c| c.revelation_type.clone())
                .or_else(|| {
                    cached
                        .revelation_place
                        .clone()
                        .map(|p| p.to_lowercase())
                })
                .unwrap_or_default(),
            total_verses: cached.total_ayah,
            description: chapter.and_then(|c| c.description.clone()),
            surah_number: n,
            audio: Self::merge_audio(cached),
        })
    }

    fn merge_audio(cached: &crate::models::CachedSurah) -> HashMap<String, ReciterAudio> {
        let mut audio: HashMap<String, ReciterAudio> = cached.audio.clone();
        let padded = format!("{:03}", cached.surah_no);
        let mut luhaidan = ReciterAudio {
            reciter: LUHAIDAN_NAME.into(),
            url: LUHAIDAN_URL.replace("{NNN}", &padded),
            original_url: None,
        };
        if LUHAIDAN_FALLBACK_SURAHS.contains(&cached.surah_no) {
            luhaidan.url = LUHAIDAN_FALLBACK_URL.replace("{NNN}", &padded);
        }
        audio.insert(LUHAIDAN_KEY.into(), luhaidan);
        audio
    }

    pub fn get_ayahs(&self, surah: u32, start: u32, end: u32) -> Vec<Ayah> {
        let cached = match self.cache.get(surah) {
            Some(c) => c,
            None => return Vec::new(),
        };
        let chapter = self.main_db.get(&surah.to_string());
        let tr = self.tr_db.get(surah as usize - 1);
        let end = end.min(cached.total_ayah);

        (start..=end)
            .map(|n| {
                let idx = n as usize - 1;
                let verse = chapter.and_then(|c| c.verses.get(&n.to_string()));
                let tr_verse = tr.and_then(|s| s.verses.get(idx));
                Ayah {
                    number: n,
                    content: cached
                        .arabic_uthmani
                        .get(idx)
                        .cloned()
                        .unwrap_or_default(),
                    transliteration: verse
                        .map(|v| v.transliteration.clone())
                        .unwrap_or_default(),
                    text: cached.english.get(idx).cloned().unwrap_or_default(),
                    translation_tr: tr_verse
                        .map(|v| v.translation.clone())
                        .unwrap_or_default(),
                }
            })
            .collect()
    }

    pub fn display_content(&self, content: &str) -> String {
        if self.arabic_reversed {
            fix_arabic_text(content).chars().rev().collect()
        } else {
            fix_arabic_text(content)
        }
    }

    pub fn get_all_surah_names(&self) -> Vec<String> {
        self.cache.surah_names()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache() -> QuranCache {
        let mut cache = QuranCache::default();
        cache.insert(
            1,
            serde_json::from_str(
                r#"{
                    "surahName": "Al-Faatiha",
                    "surahNameArabic": "الفاتحة",
                    "surahNameArabicLong": "سُورَةُ ٱلْفَاتِحَةِ",
                    "surahNameTranslation": "The Opening",
                    "revelationPlace": "Mecca",
                    "totalAyah": 7,
                    "surahNo": 1,
                    "audio": {
                        "1": {"reciter": "Mishary Rashid Al-Afasy", "url": "https://example.com/1.mp3"}
                    },
                    "english": ["In the Name of Allah", "Praise be to Allah"],
                    "arabic1": ["بِسْمِ", "الْحَمْدُ"],
                    "arabic2": ["بسم", "الحمد"]
                }"#,
            )
            .unwrap(),
        );
        cache
    }

    #[test]
    fn merges_luhaidan_into_audio() {
        let cache = make_cache();
        let handler = DataHandler {
            cache,
            main_db: HashMap::new(),
            tr_db: Vec::new(),
            arabic_reversed: false,
        };
        let info = handler.get_surah_info(1).unwrap();
        assert_eq!(info.audio.len(), 2);
        let luhaidan = &info.audio[LUHAIDAN_KEY];
        assert_eq!(luhaidan.reciter, LUHAIDAN_NAME);
        assert!(luhaidan.url.contains("muhammad_alhaidan/001.mp3"));
    }

    #[test]
    fn luhaidan_fallback_for_surah_two() {
        let mut cache = make_cache();
        cache.insert(
            2,
            serde_json::from_str(
                r#"{
                    "surahName": "Al-Baqarah",
                    "surahNameArabic": "البقرة",
                    "totalAyah": 286,
                    "surahNo": 2,
                    "audio": {},
                    "english": ["Alif Lam Meem"],
                    "arabic1": ["الم"],
                    "arabic2": ["الم"]
                }"#,
            )
            .unwrap(),
        );
        let handler = DataHandler {
            cache,
            main_db: HashMap::new(),
            tr_db: Vec::new(),
            arabic_reversed: false,
        };
        let info = handler.get_surah_info(2).unwrap();
        let luhaidan = &info.audio[LUHAIDAN_KEY];
        assert!(luhaidan.url.contains("fadsec-lab/quran-audios"));
    }

    #[test]
    fn get_ayahs_clamps_range_and_merges() {
        let cache = make_cache();
        let handler = DataHandler {
            cache,
            main_db: HashMap::new(),
            tr_db: Vec::new(),
            arabic_reversed: false,
        };
        let ayahs = handler.get_ayahs(1, 1, 99);
        assert_eq!(ayahs.len(), 7);
        assert_eq!(ayahs[0].number, 1);
        assert_eq!(ayahs[0].text, "In the Name of Allah");
        assert_eq!(ayahs[0].content, "بِسْمِ");
        assert_eq!(ayahs[6].text, "");
    }

    #[test]
    fn display_content_respects_reversal() {
        let mut handler = DataHandler {
            cache: make_cache(),
            main_db: HashMap::new(),
            tr_db: Vec::new(),
            arabic_reversed: false,
        };
        let normal = handler.display_content("بسم");
        handler.arabic_reversed = true;
        let reversed = handler.display_content("بسم");
        assert_ne!(normal, reversed);
    }
}