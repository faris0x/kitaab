use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::data::DataHandler;
use crate::ui::Input;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SubtitlePhase {
    #[default]
    Surah,
    Range,
    Confirm,
}

#[derive(Default)]
pub struct SubtitlesState {
    pub phase: SubtitlePhase,
    pub surah: Input,
    pub start: Input,
    pub end: Input,
    pub end_focus: bool,
    pub surah_number: u32,
    pub surah_name: String,
    pub total_verses: u32,
    pub error: Option<String>,
}

fn row(label: &str, value: &str, included: bool, accent: Color) -> Line<'static> {
    let status = if included {
        Span::styled("✓ Included", Style::default().fg(Color::Green))
    } else {
        Span::styled("✗ Excluded", Style::default().fg(Color::Red))
    };
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(accent)),
        Span::raw("  "),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
        Span::raw("  "),
        status,
    ])
}

pub fn render(
    frame: &mut Frame,
    state: &SubtitlesState,
    _data: &DataHandler,
    config: &crate::settings::SubtitleConfig,
    accent: Color,
) {
    let chunks = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .split(frame.area());

    let mut lines = Vec::new();
    match state.phase {
        SubtitlePhase::Surah => {
            lines.push(Line::styled(
                "Subtitle (.srt) creation",
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::default());
            lines.push(Line::styled(
                "Enter a surah number (1-114) or name:",
                Style::default().fg(Color::Gray),
            ));
        }
        SubtitlePhase::Range => {
            lines.push(Line::styled(
                format!("Subtitle creation — {}", state.surah_name),
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::default());
            lines.push(Line::styled(
                format!("Choose ayah range (1-{})", state.total_verses),
                Style::default().fg(Color::Gray),
            ));
        }
        SubtitlePhase::Confirm => {
            lines.push(Line::styled(
                "Confirm subtitle content",
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled("Surah", Style::default().fg(accent)),
                Span::raw(format!(
                    " : {} ({})",
                    state.surah_name, state.surah_number
                )),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Ayahs", Style::default().fg(accent)),
                Span::raw(format!(" : {} - {}", state.start.buffer, state.end.buffer)),
            ]));
            lines.push(Line::default());
            lines.push(row("Arabic", "(always)", true, accent));
            lines.push(row(
                "Transliteration",
                "(t)",
                config.include_transliteration,
                accent,
            ));
            lines.push(row("Turkish", "(k)", config.include_turkish, accent));
            lines.push(row("English", "(e)", config.include_english, accent));
            lines.push(Line::default());
            lines.push(Line::styled(
                "g generate  ·  t/k/e toggle  ·  Esc back",
                Style::default().fg(Color::Gray),
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded),
        ),
        chunks[0],
    );

    match state.phase {
        SubtitlePhase::Surah => {
            render_input(frame, "Surah number:", &state.surah, chunks[1], accent);
        }
        SubtitlePhase::Range => {
            if state.end_focus {
                render_input(frame, "End ayah:", &state.end, chunks[1], accent);
            } else {
                render_input(frame, "Start ayah:", &state.start, chunks[1], accent);
            }
        }
        SubtitlePhase::Confirm => {}
    }

    if let Some(error) = &state.error {
        frame.render_widget(
            Paragraph::new(Line::from(error.as_str())).style(Style::default().fg(Color::Red)),
            chunks[3],
        );
    }
}

fn render_input(frame: &mut Frame, label: &str, input: &Input, area: ratatui::layout::Rect, accent: Color) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(label, Style::default().fg(accent)),
            Span::raw(" "),
            Span::raw(input.buffer.as_str()),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded),
        ),
        area,
    );
}

pub enum SubtitlesAction {
    None,
    Back,
    SetSurah(u32),
    SetRange,
    Generate,
    ToggleTransliteration,
    ToggleTurkish,
    ToggleEnglish,
}

pub fn handle_key(
    state: &mut SubtitlesState,
    data: &DataHandler,
    key: KeyEvent,
) -> SubtitlesAction {
    match state.phase {
        SubtitlePhase::Surah => match key.code {
            KeyCode::Char(_) | KeyCode::Backspace => {
                state.surah.handle_key(key);
                state.error = None;
                SubtitlesAction::None
            }
            KeyCode::Enter => {
                let input = state.surah.buffer.trim().to_string();
                if let Ok(n) = input.parse::<u32>() {
                    if (1..=114).contains(&n) {
                        return SubtitlesAction::SetSurah(n);
                    }
                    state.error = Some("Surah must be 1-114".into());
                    return SubtitlesAction::None;
                }
                let names = data.get_all_surah_names();
                let matches = crate::fuzzy::get_close_matches(&input, &names, 3, 0.6);
                match matches.len() {
                    0 => {
                        state.error = Some("No matching surah found".into());
                        SubtitlesAction::None
                    }
                    1 => {
                        let n = names
                            .iter()
                            .position(|n| n == matches[0])
                            .map(|i| i as u32 + 1)
                            .unwrap_or(0);
                        SubtitlesAction::SetSurah(n)
                    }
                    _ => {
                        state.error = Some(format!(
                            "Did you mean: {}",
                            matches.join(", ")
                        ));
                        SubtitlesAction::None
                    }
                }
            }
            KeyCode::Esc => SubtitlesAction::Back,
            _ => SubtitlesAction::None,
        },
        SubtitlePhase::Range => match key.code {
            KeyCode::Char(_) | KeyCode::Backspace => {
                if state.end_focus {
                    state.end.handle_key(key);
                } else {
                    state.start.handle_key(key);
                }
                state.error = None;
                SubtitlesAction::None
            }
            KeyCode::Enter => {
                if !state.end_focus {
                    state.end_focus = true;
                    SubtitlesAction::None
                } else {
                    SubtitlesAction::SetRange
                }
            }
            KeyCode::Esc => {
                state.surah.clear();
                state.start.clear();
                state.end.clear();
                state.end_focus = false;
                state.phase = SubtitlePhase::Surah;
                SubtitlesAction::None
            }
            _ => SubtitlesAction::None,
        },
        SubtitlePhase::Confirm => match key.code {
            KeyCode::Char('g') => SubtitlesAction::Generate,
            KeyCode::Char('t') => SubtitlesAction::ToggleTransliteration,
            KeyCode::Char('k') => SubtitlesAction::ToggleTurkish,
            KeyCode::Char('e') => SubtitlesAction::ToggleEnglish,
            KeyCode::Esc => {
                state.phase = SubtitlePhase::Range;
                SubtitlesAction::None
            }
            _ => SubtitlesAction::None,
        },
    }
}

impl SubtitlesState {
    pub fn set_surah(&mut self, n: u32, name: String, total: u32) {
        self.surah_number = n;
        self.surah_name = name;
        self.total_verses = total;
        self.start.clear();
        self.end.clear();
        self.end_focus = false;
        self.error = None;
        self.phase = SubtitlePhase::Range;
    }
}