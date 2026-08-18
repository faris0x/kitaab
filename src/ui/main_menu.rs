use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::BANNER;
use crate::ui::Input;

pub const COMMANDS: &[(&str, &str)] = &[
    ("1-114", "Select a surah by number"),
    ("name", "Search surah by name (e.g. 'rahman')"),
    ("list / ls", "Show all 114 surahs"),
    ("sub", "Generate subtitle (.srt) files"),
    ("bookmarks / bm", "Manage bookmarks"),
    ("settings / st", "Theme, translations, audio, backup"),
    ("audio / au", "Audio cache management"),
    ("info / i", "Help and credits"),
    ("readme / rd", "Show the app readme"),
    ("quit / exit", "Exit kitaab"),
];

#[derive(Default)]
pub struct MainMenuState {
    pub input: Input,
    pub did_you_mean: Option<Vec<String>>,
    pub notice: Option<String>,
}

pub fn render(frame: &mut Frame, state: &MainMenuState, accent: Color) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    let banner_width = BANNER.lines().map(|l| l.len()).max().unwrap_or(0) as u16;
    let banner_x = (area.width.saturating_sub(banner_width)) / 2;
    let banner_lines: Vec<Line> = BANNER
        .lines()
        .map(|l| Line::from(l.trim_end()))
        .collect();
    frame.render_widget(
        Paragraph::new(banner_lines).style(Style::default().fg(accent)),
        Rect::new(banner_x, chunks[0].y, banner_width, 6),
    );

    if let Some(suggestions) = &state.did_you_mean {
        render_suggestions(frame, chunks[2], suggestions, accent);
    } else {
        render_commands(frame, chunks[2], accent);
    }

    if let Some(notice) = &state.notice {
        frame.render_widget(
            Paragraph::new(Line::from(notice.as_str())).style(Style::default().fg(Color::Yellow)),
            chunks[3],
        );
    }

    let prompt_text = if state.did_you_mean.is_some() {
        "Pick a number, or 'r' to retry:"
    } else {
        "❯"
    };
    let prompt_line = Line::from(vec![
        Span::styled(prompt_text, Style::default().fg(accent)),
        Span::raw(" "),
        Span::raw(state.input.buffer.as_str()),
        Span::styled("█", Style::default().fg(Color::Gray)),
    ]);
    frame.render_widget(
        Paragraph::new(prompt_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded),
        ),
        chunks[4],
    );
}

fn render_commands(frame: &mut Frame, area: Rect, accent: Color) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(
        "Available Commands",
        Style::default().fg(accent).add_modifier(ratatui::style::Modifier::BOLD),
    ));
    lines.push(Line::default());
    for (cmd, desc) in COMMANDS {
        lines.push(Line::from(vec![
            Span::styled(*cmd, Style::default().fg(accent)),
            Span::raw("  —  "),
            Span::raw(*desc),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded),
        ),
        area,
    );
}

fn render_suggestions(frame: &mut Frame, area: Rect, suggestions: &[String], accent: Color) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(
        "Did you mean:",
        Style::default().fg(accent).add_modifier(ratatui::style::Modifier::BOLD),
    ));
    lines.push(Line::default());
    for (i, name) in suggestions.iter().enumerate() {
        lines.push(Line::from(vec![
            Span::styled(format!("{}. ", i + 1), Style::default().fg(accent)),
            Span::raw(name.as_str()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::raw("Enter the number to select, "),
        Span::styled("r", Style::default().fg(accent)),
        Span::raw(" to search again"),
    ]));
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .border_type(BorderType::Rounded),
            ),
        area,
    );
}