use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::settings::Bookmark;
use crate::ui::Input;

#[derive(Default)]
pub struct BookmarksState {
    pub entries: Vec<(u32, Bookmark)>,
    pub selected: usize,
    pub offset: usize,
    pub adding: bool,
    pub input: Input,
}

impl BookmarksState {
    pub fn refresh(&mut self, bookmarks: &std::collections::HashMap<String, Vec<Bookmark>>) {
        let mut entries = Vec::new();
        let mut keys: Vec<u32> = bookmarks
            .keys()
            .filter_map(|k| k.parse().ok())
            .collect();
        keys.sort_unstable();
        for surah in keys {
            if let Some(list) = bookmarks.get(&surah.to_string()) {
                for b in list {
                    entries.push((surah, b.clone()));
                }
            }
        }
        self.entries = entries;
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        self.offset = self.offset.min(
            self.entries
                .len()
                .saturating_sub(10)
                .max(1)
                .saturating_sub(1),
        );
    }
}

pub fn render(
    frame: &mut Frame,
    state: &BookmarksState,
    names: &[String],
    accent: Color,
) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(frame.area());
    let list_area = chunks[0];

    let mut lines: Vec<Line> = Vec::new();
    let visible = list_area.height.saturating_sub(2) as usize;
    let offset = state.offset.min(state.entries.len().saturating_sub(visible));
    if state.entries.is_empty() && !state.adding {
        lines.push(Line::styled(
            "No bookmarks saved yet. Type 'bookmark' (or 'bm') while reading, or press 'a' here.",
            Style::default().fg(Color::DarkGray),
        ));
    }
    for (i, (surah, bookmark)) in state.entries.iter().enumerate().skip(offset).take(visible) {
        let name = names.get(*surah as usize - 1).cloned().unwrap_or_default();
        let note = if bookmark.note.is_empty() {
            String::new()
        } else {
            format!(" — {}", bookmark.note)
        };
        let line = format!(
            "Surah {surah}: {name}  ·  Ayah {} {note}  ·  {}",
            bookmark.ayah, bookmark.date_added
        );
        if i == state.selected {
            lines.push(Line::styled(
                line,
                Style::default().fg(accent).add_modifier(Modifier::REVERSED),
            ));
        } else {
            lines.push(Line::styled(line, Style::default().fg(Color::Gray)));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .title(" Bookmark Manager ")
                .border_type(BorderType::Rounded),
        ),
        list_area,
    );

    let footer: Vec<Span> = if state.adding {
        vec![
            Span::styled("Add as ", Style::default().fg(Color::Gray)),
            Span::styled("surah,ayah,note", Style::default().fg(accent)),
            Span::styled("  ·  Enter save  ·  Esc cancel", Style::default().fg(Color::Gray)),
        ]
    } else {
        vec![
            Span::styled("↑/↓", Style::default().fg(accent)),
            Span::raw(" navigate  ·  "),
            Span::styled("a", Style::default().fg(accent)),
            Span::raw(" add  ·  "),
            Span::styled("d", Style::default().fg(accent)),
            Span::raw(" delete  ·  "),
            Span::styled("q/Esc", Style::default().fg(accent)),
            Span::raw(" back"),
        ]
    };
    let line = if state.adding {
        Line::from(vec![
            Span::styled(
                format!("> {}", state.input.buffer),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(footer)
    };
    frame.render_widget(Paragraph::new(line), chunks[1]);
}

pub enum BookmarksAction {
    None,
    Back,
    Add(String),
    Delete(usize),
}

pub fn handle_key(
    state: &mut BookmarksState,
    key: KeyEvent,
) -> BookmarksAction {
    if state.adding {
        match key.code {
            KeyCode::Char(_) | KeyCode::Backspace => {
                state.input.handle_key(key);
                return BookmarksAction::None;
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut state.input.buffer);
                state.adding = false;
                if input.trim().is_empty() {
                    return BookmarksAction::None;
                }
                return BookmarksAction::Add(input);
            }
            KeyCode::Esc => {
                state.adding = false;
                state.input.clear();
                return BookmarksAction::None;
            }
            _ => return BookmarksAction::None,
        }
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => BookmarksAction::Back,
        KeyCode::Char('a') => {
            state.adding = true;
            state.input.clear();
            BookmarksAction::None
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            if !state.entries.is_empty() {
                BookmarksAction::Delete(state.selected)
            } else {
                BookmarksAction::None
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !state.entries.is_empty() {
                state.selected = (state.selected + 1).min(state.entries.len() - 1);
            }
            BookmarksAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            state.selected = state.selected.saturating_sub(1);
            BookmarksAction::None
        }
        _ => BookmarksAction::None,
    }
}

pub fn parse_add_input(input: &str) -> Result<(u32, u32, String), String> {
    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() < 2 {
        return Err("Invalid input. Use surah,ayah,note (e.g., 2,255,My note)".into());
    }
    let surah = parts[0]
        .trim()
        .parse::<u32>()
        .map_err(|_| "Surah must be a number (1-114)".to_string())?;
    if !(1..=114).contains(&surah) {
        return Err("Surah must be 1-114".into());
    }
    let ayah = parts[1]
        .trim()
        .parse::<u32>()
        .map_err(|_| "Ayah must be a number".to_string())?;
    let note = parts
        .get(2)
        .map(|n| n.trim().chars().take(300).collect())
        .unwrap_or_default();
    Ok((surah, ayah, note))
}
