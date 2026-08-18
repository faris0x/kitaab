use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme_color: String,
    pub reading_config: ReadingConfig,
    pub subtitle_config: SubtitleConfig,
    pub bookmarks: HashMap<String, Vec<Bookmark>>,
    pub ayatul_kursi: Option<ReciterPref>,
    #[serde(flatten)]
    pub surah_reciters: HashMap<String, ReciterPref>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_color: "red".into(),
            reading_config: ReadingConfig::default(),
            subtitle_config: SubtitleConfig::default(),
            bookmarks: HashMap::new(),
            ayatul_kursi: None,
            surah_reciters: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadingConfig {
    pub show_turkish: bool,
    pub show_english: bool,
    pub show_transliteration: bool,
}

impl Default for ReadingConfig {
    fn default() -> Self {
        Self {
            show_turkish: false,
            show_english: true,
            show_transliteration: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleConfig {
    pub include_turkish: bool,
    pub include_english: bool,
    pub include_transliteration: bool,
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            include_turkish: false,
            include_english: true,
            include_transliteration: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub ayah: u32,
    pub note: String,
    pub date_added: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciterPref {
    pub reciter_name: String,
    pub reciter_url: String,
}

fn format_datetime() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = now.div_euclid(86_400);
    let secs_of_day = now.rem_euclid(86_400);
    let (hour, minute) = ((secs_of_day / 3600) as u32, (secs_of_day % 3600 / 60) as u32);
    let weekday = ["Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday"]
        [(days.rem_euclid(7)) as usize];
    let mut year = 1970;
    let mut remaining = days;
    loop {
        let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    while month < 11 && remaining >= month_days[month] {
        remaining -= month_days[month];
        month += 1;
    }
    let day = remaining + 1;
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][month];
    let hour12 = hour % 12;
    let hour12 = if hour12 == 0 { 12 } else { hour12 };
    let period = if hour < 12 { "AM" } else { "PM" };
    format!(
        "{weekday}, {day:02} {month_name} {year} {hour12:02}:{minute:02} {period}"
    )
}

impl Settings {
    pub fn add_bookmark(&mut self, surah: u32, ayah: u32, note: String) {
        let entries = self.bookmarks.entry(surah.to_string()).or_default();
        if let Some(existing) = entries.iter_mut().find(|b| b.ayah == ayah) {
            existing.note = note;
            existing.date_added = format_datetime();
            return;
        }
        entries.push(Bookmark {
            ayah,
            note,
            date_added: format_datetime(),
        });
    }

    pub fn remove_bookmark(&mut self, surah: u32, ayah: Option<u32>) {
        let Some(entries) = self.bookmarks.get_mut(&surah.to_string()) else {
            return;
        };
        if let Some(ayah) = ayah {
            entries.retain(|b| b.ayah != ayah);
        } else {
            entries.clear();
        }
        if entries.is_empty() {
            self.bookmarks.remove(&surah.to_string());
        }
    }
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn surah_reciter(&self, surah: u32) -> Option<&ReciterPref> {
        self.surah_reciters.get(&surah.to_string())
    }

    #[allow(dead_code)]
    pub fn set_surah_reciter(&mut self, surah: u32, pref: ReciterPref) {
        self.surah_reciters.insert(surah.to_string(), pref);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_original() {
        let settings = Settings::default();
        assert_eq!(settings.theme_color, "red");
        assert!(!settings.reading_config.show_turkish);
        assert!(settings.reading_config.show_english);
        assert!(settings.reading_config.show_transliteration);
        assert!(!settings.subtitle_config.include_turkish);
        assert!(settings.subtitle_config.include_english);
        assert!(!settings.subtitle_config.include_transliteration);
    }

    #[test]
    fn roundtrips_through_json() {
        let mut settings = Settings::default();
        settings.theme_color = "green".into();
        settings.reading_config.show_turkish = true;
        settings.set_surah_reciter(
            2,
            ReciterPref {
                reciter_name: "Abu Bakr Al-Shatri".into(),
                reciter_url: "https://example.com/2.mp3".into(),
            },
        );
        settings.bookmarks.insert(
            "1".into(),
            vec![Bookmark {
                ayah: 1,
                note: "bismillah".into(),
                date_added: "Monday, 01 Jan 2026 12:00 AM".into(),
            }],
        );
        settings.ayatul_kursi = Some(ReciterPref {
            reciter_name: "Nasser Al-Qatami".into(),
            reciter_url: "https://example.com/kursi.mp3".into(),
        });

        let json = serde_json::to_string_pretty(&settings).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme_color, "green");
        assert!(parsed.reading_config.show_turkish);
        let reciter = parsed.surah_reciter(2).unwrap();
        assert_eq!(reciter.reciter_name, "Abu Bakr Al-Shatri");
        assert_eq!(parsed.bookmarks["1"][0].note, "bismillah");
        assert!(parsed.ayatul_kursi.is_some());
    }

    #[test]
    fn load_missing_file_gives_defaults() {
        let path = std::env::temp_dir().join(format!(
            "kitaab-settings-missing-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let settings = Settings::load(&path);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn save_and_load_file() {
        let path = std::env::temp_dir().join(format!(
            "kitaab-settings-roundtrip-{}.json",
            std::process::id()
        ));
        let mut settings = Settings::default();
        settings.theme_color = "blue".into();
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path);
        assert_eq!(loaded.theme_color, "blue");
        let _ = std::fs::remove_file(&path);
    }
}