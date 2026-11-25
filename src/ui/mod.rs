// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

pub mod bookmarks;
pub mod main_menu;
pub mod reader;
pub mod settings;
pub mod splash;
pub mod subtitles;
pub mod surah_list;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

pub const THEMES: &[&str] = &["red", "white", "green", "blue", "yellow", "magenta", "cyan"];

pub fn theme_color(name: &str) -> Color {
    match name {
        "white" => Color::White,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        _ => Color::Red,
    }
}

pub fn next_theme(name: &str, forward: bool) -> &'static str {
    let index = THEMES.iter().position(|theme| *theme == name).unwrap_or(0);
    let next = if forward {
        (index + 1) % THEMES.len()
    } else {
        (index + THEMES.len() - 1) % THEMES.len()
    };
    THEMES[next]
}

#[derive(Default)]
pub struct Input {
    pub buffer: String,
}

impl Input {
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.buffer.push(c),
            KeyCode::Backspace => {
                self.buffer.pop();
            }
            _ => {}
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}