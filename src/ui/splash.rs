// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph};
use ratatui::Frame;

use crate::BANNER;

pub struct SplashData {
    pub phase: &'static str,
    pub current: String,
    pub done: u32,
    pub total: u32,
    pub failed: u32,
    pub fatal: Option<String>,
}

pub fn render(frame: &mut Frame, splash: &SplashData, accent: Color) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(0),
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
        ratatui::layout::Rect::new(banner_x, chunks[0].y, banner_width, 6),
    );

    frame.render_widget(
        Paragraph::new(Line::from("Preparing first-run data…"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray)),
        chunks[1],
    );

    let (label, progress) = if splash.total == 0 {
        ("Downloading databases…".to_string(), 0.0)
    } else {
        let progress = splash.done as f64 / splash.total as f64;
        let label = format!(
            "{}: {}/{} fetched",
            splash.phase, splash.done, splash.total
        );
        (label, progress)
    };
    frame.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(label))
            .gauge_style(Style::default().fg(accent))
            .ratio(progress),
        chunks[2],
    );

    let status = if splash.failed > 0 {
        format!("{} ({} failed, will retry on next launch)", splash.current, splash.failed)
    } else {
        splash.current.clone()
    };
    frame.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Center),
        chunks[3],
    );

    if let Some(fatal) = &splash.fatal {
        let error_chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
            .split(chunks[5]);
        frame.render_widget(
            Paragraph::new(Line::from(fatal.as_str()))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Red)),
            error_chunks[0],
        );
        frame.render_widget(
            Paragraph::new(Line::from("Press 'q' to quit."))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(accent))
                        .border_type(BorderType::Rounded),
                ),
            error_chunks[1],
        );
    }
}