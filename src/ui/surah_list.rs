// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub const COLUMNS: usize = 5;

#[derive(Default)]
pub struct SurahListState {
    pub selected: usize,
    pub offset: usize,
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

pub fn render(frame: &mut Frame, state: &SurahListState, names: &[String], accent: Color) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);
    let list_area = chunks[0];
    let inner = ratatui::layout::Rect {
        x: list_area.x + 1,
        y: list_area.y + 1,
        width: list_area.width.saturating_sub(2),
        height: list_area.height.saturating_sub(2),
    };
    let col_width = inner.width / COLUMNS as u16;
    let rows_visible = inner.height as usize;
    let total_rows = names.len().div_ceil(COLUMNS);
    let offset = state
        .offset
        .min(total_rows.saturating_sub(rows_visible));

    let mut lines: Vec<Line> = Vec::new();
    for r in 0..rows_visible {
        let base = (offset + r) * COLUMNS;
        let spans: Vec<Span> = (0..COLUMNS)
            .map(|c| {
                let idx = base + c;
                if idx >= names.len() {
                    return Span::raw(" ".repeat(col_width as usize));
                }
                let cell = truncate(&format!("{:>3}. {}", idx + 1, names[idx]), col_width as usize);
                if idx == state.selected {
                    Span::styled(
                        cell,
                        Style::default()
                            .fg(accent)
                            .add_modifier(Modifier::REVERSED),
                    )
                } else {
                    Span::raw(cell)
                }
            })
            .collect();
        lines.push(Line::from(spans));
    }

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .title(" Surahs (1-114) ")
                .border_type(BorderType::Rounded),
        ),
        list_area,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(accent)),
            Span::raw(" row  ·  "),
            Span::styled("←/→", Style::default().fg(accent)),
            Span::raw(" column  ·  "),
            Span::styled("Enter", Style::default().fg(accent)),
            Span::raw(" open  ·  "),
            Span::styled("q/Esc", Style::default().fg(accent)),
            Span::raw(" back"),
        ])),
        chunks[1],
    );
}