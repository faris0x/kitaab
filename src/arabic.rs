// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use unicode_bidi::BidiInfo;

pub fn fix_arabic_text(text: &str) -> String {
    let shaped = arabic_reshaper::arabic_reshape(text);
    if shaped.is_empty() {
        return shaped;
    }
    let info = BidiInfo::new(&shaped, None);
    let mut out = String::with_capacity(shaped.len());
    for para in &info.paragraphs {
        out.push_str(&info.reorder_line(para, para.range.clone()));
    }
    out
}

#[allow(dead_code)]
pub fn raw_arabic(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshapes_and_reorders_pure_arabic() {
        let text = "الفاتحة";
        let fixed = fix_arabic_text(text);
        assert_eq!(fixed.chars().count(), text.chars().count());
        assert!(fixed.chars().all(|c| is_arabic_char(c)));
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(fix_arabic_text(""), "");
    }

    #[test]
    fn latin_text_is_left_alone() {
        let fixed = fix_arabic_text("Al-Fatihah");
        assert_eq!(fixed, "Al-Fatihah");
    }

    #[test]
    fn mixed_text_handles_embedded_runs() {
        let text = "سُورَةُ 1: بسم الله";
        let shaped = arabic_reshaper::arabic_reshape(text);
        let fixed = fix_arabic_text(text);
        assert_eq!(fixed.chars().count(), shaped.chars().count());
        assert!(fixed.contains('1'));
        assert!(fixed.chars().any(|c| is_arabic_char(c)));
    }

    fn is_arabic_char(c: char) -> bool {
        matches!(c as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
    }
}