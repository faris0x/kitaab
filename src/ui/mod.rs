pub mod bookmarks;
pub mod main_menu;
pub mod reader;
pub mod splash;
pub mod subtitles;
pub mod surah_list;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::Color;

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