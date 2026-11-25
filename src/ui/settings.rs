// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::settings::Settings;
use crate::ui::next_theme;

#[derive(Default)]
pub struct SettingsState {
    pub selected: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingItem {
    Theme,
    ReadingEnglish,
    ReadingTransliteration,
    ReadingTurkish,
    SubtitleEnglish,
    SubtitleTransliteration,
    SubtitleTurkish,
}

const ITEMS: [SettingItem; 7] = [
    SettingItem::Theme,
    SettingItem::ReadingEnglish,
    SettingItem::ReadingTransliteration,
    SettingItem::ReadingTurkish,
    SettingItem::SubtitleEnglish,
    SettingItem::SubtitleTransliteration,
    SettingItem::SubtitleTurkish,
];

pub enum SettingsAction {
    None,
    Back,
    Changed,
}

fn toggle(item: SettingItem, settings: &mut Settings) {
    match item {
        SettingItem::Theme => {
            settings.theme_color = next_theme(&settings.theme_color, true).to_string();
        }
        SettingItem::ReadingEnglish => {
            settings.reading_config.show_english = !settings.reading_config.show_english;
        }
        SettingItem::ReadingTransliteration => {
            settings.reading_config.show_transliteration =
                !settings.reading_config.show_transliteration;
        }
        SettingItem::ReadingTurkish => {
            settings.reading_config.show_turkish = !settings.reading_config.show_turkish;
        }
        SettingItem::SubtitleEnglish => {
            settings.subtitle_config.include_english =
                !settings.subtitle_config.include_english;
        }
        SettingItem::SubtitleTransliteration => {
            settings.subtitle_config.include_transliteration =
                !settings.subtitle_config.include_transliteration;
        }
        SettingItem::SubtitleTurkish => {
            settings.subtitle_config.include_turkish =
                !settings.subtitle_config.include_turkish;
        }
    }
}

fn change(item: SettingItem, settings: &mut Settings, forward: bool) {
    if item == SettingItem::Theme {
        settings.theme_color = next_theme(&settings.theme_color, forward).to_string();
    } else {
        toggle(item, settings);
    }
}

pub fn handle_key(
    state: &mut SettingsState,
    settings: &mut Settings,
    key: KeyEvent,
) -> SettingsAction {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => SettingsAction::Back,
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = state.selected.saturating_sub(1);
            SettingsAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.selected = (state.selected + 1).min(ITEMS.len() - 1);
            SettingsAction::None
        }
        KeyCode::Left | KeyCode::Char('h') => {
            change(ITEMS[state.selected], settings, false);
            SettingsAction::Changed
        }
        KeyCode::Right | KeyCode::Char('l') => {
            change(ITEMS[state.selected], settings, true);
            SettingsAction::Changed
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            toggle(ITEMS[state.selected], settings);
            SettingsAction::Changed
        }
        _ => SettingsAction::None,
    }
}

fn value(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

fn row(label: &str, value_text: &str, selected: bool, accent: Color) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(accent)
            .add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(Color::Gray)
    };
    Line::from(vec![
        Span::styled(if selected { "> " } else { "  " }, style),
        Span::styled(label.to_string(), style),
        Span::raw("  "),
        Span::styled(value_text.to_string(), style),
    ])
}

fn heading(text: &str, accent: Color) -> Line<'static> {
    Line::styled(
        text.to_string(),
        Style::default()
            .fg(accent)
            .add_modifier(Modifier::BOLD),
    )
}

pub fn render(
    frame: &mut Frame,
    state: &SettingsState,
    settings: &Settings,
    accent: Color,
) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(frame.area());
    let selected = state.selected.min(ITEMS.len() - 1);
    let mut lines = Vec::new();

    lines.push(heading("Appearance", accent));
    lines.push(row(
        "Theme",
        &settings.theme_color,
        selected == 0,
        accent,
    ));
    lines.push(Line::default());
    lines.push(heading("Reading", accent));
    lines.push(row(
        "English translation",
        value(settings.reading_config.show_english),
        selected == 1,
        accent,
    ));
    lines.push(row(
        "Transliteration",
        value(settings.reading_config.show_transliteration),
        selected == 2,
        accent,
    ));
    lines.push(row(
        "Turkish translation",
        value(settings.reading_config.show_turkish),
        selected == 3,
        accent,
    ));
    lines.push(Line::default());
    lines.push(heading("Subtitles", accent));
    lines.push(row(
        "English translation",
        value(settings.subtitle_config.include_english),
        selected == 4,
        accent,
    ));
    lines.push(row(
        "Transliteration",
        value(settings.subtitle_config.include_transliteration),
        selected == 5,
        accent,
    ));
    lines.push(row(
        "Turkish translation",
        value(settings.subtitle_config.include_turkish),
        selected == 6,
        accent,
    ));

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .title(" Settings ")
                .border_type(BorderType::Rounded),
        ),
        chunks[0],
    );

    let footer = Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(accent)),
        Span::raw(" select  ·  "),
        Span::styled("←/→", Style::default().fg(accent)),
        Span::raw(" change  ·  "),
        Span::styled("Enter/Space", Style::default().fg(accent)),
        Span::raw(" toggle  ·  "),
        Span::styled("q/Esc", Style::default().fg(accent)),
        Span::raw(" back"),
    ]);
    let note = Line::styled(
        "Changes save automatically.",
        Style::default().fg(Color::DarkGray),
    );
    frame.render_widget(Paragraph::new(vec![footer, note]), chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn theme_cycles_in_both_directions() {
        assert_eq!(next_theme("red", true), "white");
        assert_eq!(next_theme("red", false), "cyan");
        assert_eq!(next_theme("cyan", true), "red");
    }

    #[test]
    fn enter_toggles_the_selected_setting() {
        let mut state = SettingsState { selected: 1 };
        let mut settings = Settings::default();
        let before = settings.reading_config.show_english;
        let action = handle_key(
            &mut state,
            &mut settings,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert!(matches!(action, SettingsAction::Changed));
        assert_ne!(settings.reading_config.show_english, before);
    }

    #[test]
    fn selection_stops_at_the_list_bounds() {
        let mut state = SettingsState::default();
        let mut settings = Settings::default();
        handle_key(
            &mut state,
            &mut settings,
            KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        );
        assert_eq!(state.selected, 0);
        for _ in 0..20 {
            handle_key(
                &mut state,
                &mut settings,
                KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            );
        }
        assert_eq!(state.selected, ITEMS.len() - 1);
    }

    #[test]
    fn toggles_each_setting() {
        let mut settings = Settings::default();
        for item in ITEMS {
            let before_theme = settings.theme_color.clone();
            let before_reading = settings.reading_config.clone();
            let before_subtitles = settings.subtitle_config.clone();
            toggle(item, &mut settings);

            if item == SettingItem::Theme {
                assert_ne!(settings.theme_color, before_theme);
            } else {
                assert!(
                    settings.reading_config != before_reading
                        || settings.subtitle_config != before_subtitles
                );
            }
        }
    }
}
