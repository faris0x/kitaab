use std::path::Path;

use anyhow::Result;

use crate::data::{Ayah, DataHandler};
use crate::settings::SubtitleConfig;

pub const AYAH_DURATION: f64 = 5.0;

pub fn format_time_srt(seconds: f64) -> String {
    let ms_total = (seconds * 1000.0) as i64;
    let hours = ms_total / 3_600_000;
    let minutes = ms_total % 3_600_000 / 60_000;
    let seconds = ms_total % 60_000 / 1000;
    let ms = ms_total % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{ms:03}")
}

pub fn generate_srt_content(
    ayahs: &[Ayah],
    config: &SubtitleConfig,
) -> String {
    let mut srt = String::new();
    for (i, ayah) in ayahs.iter().enumerate() {
        let start = i as f64 * AYAH_DURATION;
        let end = start + AYAH_DURATION;
        srt.push_str(&format!("{}\n", i + 1));
        srt.push_str(&format!(
            "{} --> {}\n",
            format_time_srt(start),
            format_time_srt(end)
        ));
        srt.push_str(&ayah.content);
        srt.push('\n');
        if config.include_transliteration && !ayah.transliteration.is_empty() {
            srt.push_str(&ayah.transliteration);
            srt.push('\n');
        }
        if config.include_turkish && !ayah.translation_tr.is_empty() {
            srt.push_str(&ayah.translation_tr);
            srt.push('\n');
        }
        if config.include_english && !ayah.text.is_empty() {
            srt.push_str(&ayah.text);
            srt.push('\n');
        }
        srt.push('\n');
    }
    srt.trim_end().to_string()
}

pub fn today_ymd() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = now.div_euclid(86_400);
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
    format!("{year}-{:02}-{:02}", month + 1, remaining + 1)
}

pub fn generate_srt_file(
    data: &DataHandler,
    subtitle_dir: &Path,
    surah: u32,
    surah_name: &str,
    start: u32,
    end: u32,
    config: &SubtitleConfig,
) -> Result<std::path::PathBuf> {
    let ayahs = data.get_ayahs(surah, start, end);
    if ayahs.is_empty() {
        anyhow::bail!("no ayah data for surah {surah}");
    }
    let content = generate_srt_content(&ayahs, config);
    if content.is_empty() {
        anyhow::bail!("generated empty SRT content");
    }
    let surah_dir = subtitle_dir.join(surah_name);
    std::fs::create_dir_all(&surah_dir)?;
    let filename = format!(
        "Surah{surah:03}_Ayah{start:03}-{end:03}_{}.srt",
        today_ymd()
    );
    let path = surah_dir.join(filename);
    std::fs::write(&path, content)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ayahs() -> Vec<Ayah> {
        vec![
            Ayah {
                number: 1,
                content: "بِسْمِ".into(),
                transliteration: "bismi".into(),
                text: "In the Name".into(),
                translation_tr: "Rahman adıyla".into(),
            },
            Ayah {
                number: 2,
                content: "الْحَمْدُ".into(),
                transliteration: "alhamdu".into(),
                text: "Praise".into(),
                translation_tr: "Hamd".into(),
            },
        ]
    }

    #[test]
    fn srt_timestamps_advance_by_five_seconds() {
        assert_eq!(format_time_srt(0.0), "00:00:00,000");
        assert_eq!(format_time_srt(5.0), "00:00:05,000");
        assert_eq!(format_time_srt(3600.0), "01:00:00,000");
    }

    #[test]
    fn srt_content_orders_arabic_translit_turkish_english() {
        let config = SubtitleConfig {
            include_turkish: true,
            include_english: true,
            include_transliteration: true,
        };
        let srt = generate_srt_content(&sample_ayahs(), &config);
        let blocks: Vec<&str> = srt.split("\n\n").collect();
        assert_eq!(blocks.len(), 2);
        let first: Vec<&str> = blocks[0].lines().collect();
        assert_eq!(first[0], "1");
        assert!(first[1].starts_with("00:00:00,000 --> 00:00:05,000"));
        assert_eq!(first[2], "بِسْمِ");
        assert_eq!(first[3], "bismi");
        assert_eq!(first[4], "Rahman adıyla");
        assert_eq!(first[5], "In the Name");
    }

    #[test]
    fn srt_omits_disabled_languages() {
        let config = SubtitleConfig {
            include_turkish: false,
            include_english: false,
            include_transliteration: false,
        };
        let srt = generate_srt_content(&sample_ayahs(), &config);
        let blocks: Vec<&str> = srt.split("\n\n").collect();
        let first: Vec<&str> = blocks[0].lines().collect();
        assert_eq!(first.len(), 3);
        assert_eq!(first[0], "1");
        assert_eq!(first[2], "بِسْمِ");
        assert!(!srt.contains("bismi"));
        assert!(!srt.contains("Praise"));
    }
}