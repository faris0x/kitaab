use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSurah {
    #[serde(rename = "surahName")]
    pub surah_name: String,
    #[serde(rename = "surahNameArabic")]
    pub surah_name_arabic: String,
    #[serde(rename = "surahNameArabicLong")]
    pub surah_name_arabic_long: Option<String>,
    #[serde(rename = "surahNameTranslation")]
    pub surah_name_translation: Option<String>,
    #[serde(rename = "revelationPlace")]
    pub revelation_place: Option<String>,
    #[serde(rename = "totalAyah")]
    pub total_ayah: u32,
    #[serde(rename = "surahNo")]
    pub surah_no: u32,
    pub audio: HashMap<String, ReciterAudio>,
    pub english: Vec<String>,
    #[serde(rename = "arabic1")]
    pub arabic_uthmani: Vec<String>,
    #[serde(rename = "arabic2")]
    pub arabic_simple: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReciterAudio {
    pub reciter: String,
    pub url: String,
    #[serde(rename = "originalUrl")]
    pub original_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TranslationDb {
    #[serde(default)]
    pub total_surahs: Option<u32>,
    pub chapters: HashMap<String, Chapter>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Chapter {
    pub id: u32,
    pub surah_name: String,
    pub surah_name_ar: String,
    pub translation: String,
    #[serde(rename = "type")]
    pub revelation_type: String,
    pub total_verses: u32,
    #[serde(default)]
    pub description: Option<String>,
    pub verses: HashMap<String, Verse>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Verse {
    pub id: f64,
    pub content: String,
    pub translation_eng: String,
    pub transliteration: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SurahTranslation {
    pub id: u32,
    pub name: String,
    pub transliteration: String,
    pub translation: String,
    #[serde(rename = "type")]
    pub revelation_type: String,
    pub total_verses: u32,
    pub verses: Vec<TranslatedVerse>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TranslatedVerse {
    pub id: u32,
    pub text: String,
    pub translation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cache_surah() {
        let json = r#"{
            "surahName": "Al-Faatiha",
            "surahNameArabic": "الفاتحة",
            "surahNameArabicLong": "سُورَةُ ٱلْفَاتِحَةِ",
            "surahNameTranslation": "The Opening",
            "revelationPlace": "Mecca",
            "totalAyah": 7,
            "surahNo": 1,
            "audio": {
                "1": {
                    "reciter": "Mishary Rashid Al-Afasy",
                    "url": "https://example.com/1.mp3",
                    "originalUrl": "https://example.com/orig.mp3"
                }
            },
            "english": ["In the Name of Allah"],
            "arabic1": ["بِسْمِ ٱللَّهِ"],
            "arabic2": ["بسم الله"]
        }"#;
        let surah: CachedSurah = serde_json::from_str(json).unwrap();
        assert_eq!(surah.surah_name, "Al-Faatiha");
        assert_eq!(surah.total_ayah, 7);
        assert_eq!(surah.english.len(), 1);
        let audio = surah.audio.get("1").unwrap();
        assert_eq!(audio.reciter, "Mishary Rashid Al-Afasy");
    }

    #[test]
    fn parses_main_db_chapter() {
        let json = r#"{
            "chapters": {
                "1": {
                    "id": 1,
                    "surah_name": "AL-FĀTIḤAH",
                    "surah_name_ar": "الفاتحة",
                    "translation": "THE OPENING",
                    "type": "meccan",
                    "total_verses": 7,
                    "description": "The opening surah.",
                    "verses": {
                        "1": {
                            "id": 1.1,
                            "content": "بِسمِ",
                            "translation_eng": "In the Name",
                            "transliteration": "bi-smi"
                        }
                    }
                }
            }
        }"#;
        let db: TranslationDb = serde_json::from_str(json).unwrap();
        let chapter = db.chapters.get("1").unwrap();
        assert_eq!(chapter.surah_name, "AL-FĀTIḤAH");
        assert_eq!(chapter.revelation_type, "meccan");
        assert_eq!(chapter.verses.get("1").unwrap().transliteration, "bi-smi");
    }

    #[test]
    fn parses_turkish_translation_db() {
        let json = r#"[{
            "id": 1,
            "name": "الفاتحة",
            "transliteration": "Al-Fatihah",
            "translation": "Açılış",
            "type": "meccan",
            "total_verses": 7,
            "verses": [
                {"id": 1, "text": "بِسۡمِ", "translation": "Rahman ve Rahim olan Allah'ın adıyla"}
            ]
        }]"#;
        let db: Vec<SurahTranslation> = serde_json::from_str(json).unwrap();
        assert_eq!(db[0].verses[0].translation, "Rahman ve Rahim olan Allah'ın adıyla");
    }
}