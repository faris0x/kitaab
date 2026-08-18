use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::arabic::fix_arabic_text;
use crate::data::{Ayah, DataHandler, SurahInfo};
use crate::settings::ReadingConfig;
use crate::ui::Input;

pub struct ReaderState {
    pub surah: u32,
    pub info: SurahInfo,
    pub phase: ReaderPhase,
    pub command_buffer: String,
    pub audio_status: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Start,
    End,
}

pub enum ReaderPhase {
    RangePrompt {
        asking_another: bool,
        focus: Focus,
        start: Input,
        end: Input,
        error: Option<String>,
    },
    Reading { pages: Vec<Vec<Ayah>>, page: usize },
    BookmarkPrompt {
        ayahs: Vec<u32>,
        step: BookmarkStep,
        ayah_input: Input,
        note_input: Input,
        error: Option<String>,
        saved: (Vec<Vec<Ayah>>, usize),
    },
}

#[derive(Clone, Copy, PartialEq)]
pub enum BookmarkStep {
    Ayah,
    Note,
}

impl ReaderState {
    pub fn new(surah: u32, info: SurahInfo) -> Self {
        Self {
            surah,
            info,
            phase: ReaderPhase::RangePrompt {
                asking_another: false,
                focus: Focus::Start,
                start: Input::default(),
                end: Input::default(),
                error: None,
            },
            command_buffer: String::new(),
            audio_status: None,
        }
    }

    #[allow(dead_code)]
    pub fn is_reading(&self) -> bool {
        matches!(self.phase, ReaderPhase::Reading { .. })
    }

    pub fn begin_reading(&mut self, data: &DataHandler) {
        let (start, end) = match &self.phase {
            ReaderPhase::RangePrompt { start, end, .. } => (start, end),
            _ => return,
        };
        let total = self.info.total_verses;
        let parse = |input: &str, default: u32| -> Option<u32> {
            let trimmed = input.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") {
                Some(default)
            } else {
                trimmed.parse().ok()
            }
        };
        let Some(start) = parse(&start.buffer, 1) else {
            self.set_range_error(format!("Start ayah must be a number (1-{total})"));
            return;
        };
        let Some(end) = parse(&end.buffer, total) else {
            self.set_range_error(format!("End ayah must be a number ({start}-{total})"));
            return;
        };
        if start < 1 || start > total || end < start || end > total {
            self.set_range_error(format!(
                "Invalid range: start must be 1-{total}, end must be >= start and <= {total}"
            ));
            return;
        }
        let ayahs = data.get_ayahs(self.surah, start, end);
        if ayahs.is_empty() {
            self.set_range_error("This surah is not available yet (failed to fetch)".into());
            return;
        }
        let page_size = page_size();
        let pages: Vec<Vec<Ayah>> = ayahs.chunks(page_size).map(|c| c.to_vec()).collect();
        self.phase = ReaderPhase::Reading { pages, page: 0 };
    }

    fn set_range_error(&mut self, error: String) {
        if let ReaderPhase::RangePrompt { error: slot, .. } = &mut self.phase {
            *slot = Some(error);
        }
    }

    pub fn next_page(&mut self) -> bool {
        if let ReaderPhase::Reading { pages, page } = &mut self.phase {
            if *page + 1 < pages.len() {
                *page += 1;
                return false;
            }
            return true;
        }
        false
    }

    pub fn prev_page(&mut self) {
        if let ReaderPhase::Reading { page, .. } = &mut self.phase
            && *page > 0
        {
            *page -= 1;
        }
    }

    pub fn to_another_range(&mut self) {
        self.phase = ReaderPhase::RangePrompt {
            asking_another: true,
            focus: Focus::Start,
            start: Input::default(),
            end: Input::default(),
            error: None,
        };
        self.command_buffer.clear();
    }

    pub fn begin_bookmark(&mut self) {
        if let ReaderPhase::Reading { pages, page } = &self.phase {
            let ayahs = pages[*page].iter().map(|a| a.number).collect();
            let saved = (pages.clone(), *page);
            self.phase = ReaderPhase::BookmarkPrompt {
                ayahs,
                step: BookmarkStep::Ayah,
                ayah_input: Input::default(),
                note_input: Input::default(),
                error: None,
                saved,
            };
        }
    }

    pub fn push_command_char(&mut self, c: char) {
        self.command_buffer.push(c);
    }

    pub fn finish_bookmark(&mut self) {
        if let ReaderPhase::BookmarkPrompt { saved, .. } = &self.phase {
            let (pages, page) = saved.clone();
            self.phase = ReaderPhase::Reading { pages, page };
        }
    }

    pub fn clear_command(&mut self) {
        self.command_buffer.clear();
    }
}

fn page_size() -> usize {
    let (rows, _) = crossterm::terminal::size().unwrap_or((24, 80));
    (rows as usize).saturating_sub(10).max(1) / 6
}

pub fn render(
    frame: &mut Frame,
    state: &mut ReaderState,
    data: &DataHandler,
    config: &ReadingConfig,
    accent: Color,
) {
    match &state.phase {
        ReaderPhase::RangePrompt {
            asking_another,
            focus,
            start,
            end,
            error,
        } => render_range_prompt(
            frame,
            state,
            asking_another,
            *focus,
            start,
            end,
            error.as_deref(),
            accent,
        ),
        ReaderPhase::Reading { .. } => render_reading(frame, state, data, config, accent),
        ReaderPhase::BookmarkPrompt { .. } => render_bookmark_prompt(frame, state, accent),
    }
}

fn render_bookmark_prompt(frame: &mut Frame, state: &ReaderState, accent: Color) {
    let ReaderPhase::BookmarkPrompt {
        ayahs,
        step,
        ayah_input,
        note_input,
        error,
        saved: _,
    } = &state.phase
    else {
        return;
    };
    let chunks = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            format!("Surah {}: ", state.info.surah_number),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            state.info.surah_name.clone(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  ·  "),
        Span::styled(
            "Bookmark an ayah from this page:",
            Style::default().fg(Color::Gray),
        ),
    ]));
    let row: Vec<Span> = ayahs
        .iter()
        .map(|n| Span::styled(format!("{n} "), Style::default().fg(Color::Cyan)))
        .collect();
    lines.push(Line::from(row));
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent))
                .border_type(BorderType::Rounded),
        ),
        chunks[0],
    );

    frame.render_widget(
        input_line(
            format!("Ayah number ({}):", state.info.total_verses),
            ayah_input,
            *step == BookmarkStep::Ayah,
            accent,
        ),
        chunks[1],
    );
    frame.render_widget(
        input_line(
            "Note (optional, Enter to save):".to_string(),
            note_input,
            *step == BookmarkStep::Note,
            accent,
        ),
        chunks[2],
    );
    if let Some(error) = error {
        frame.render_widget(
            Paragraph::new(Line::from(error.as_str())).style(Style::default().fg(Color::Red)),
            chunks[3],
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::styled(
                "Esc cancels",
                Style::default().fg(Color::DarkGray),
            )),
            chunks[3],
        );
    }
}

fn input_line(label: String, input: &Input, focused: bool, accent: Color) -> Paragraph<'static> {
    let style = if focused {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Paragraph::new(
        Line::from(vec![
            Span::styled(label, Style::default().fg(accent)),
            Span::raw(" "),
            Span::raw(input.buffer.clone()),
            if focused {
                Span::styled("█", Style::default().fg(Color::Gray))
            } else {
                Span::raw("")
            },
        ])
        .style(style),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent))
            .border_type(BorderType::Rounded),
    )
}

fn render_range_prompt(
    frame: &mut Frame,
    state: &ReaderState,
    asking_another: &bool,
    focus: Focus,
    start: &Input,
    end: &Input,
    error: Option<&str>,
    accent: Color,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    let info = &state.info;
    let mut info_lines = Vec::new();
    info_lines.push(Line::from(vec![
        Span::styled(
            format!("Surah {}: ", info.surah_number),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            info.surah_name.clone(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            fix_arabic_text(&info.surah_name_ar),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    info_lines.push(Line::styled(
        format!(
            "{}  ·  {} · {} ayahs",
            info.translation,
            info.revelation_type,
            info.total_verses
        ),
        Style::default().fg(Color::Gray),
    ));
    info_lines.push(Line::default());
    if let Some(description) = &info.description {
        info_lines.push(Line::styled(
            description.clone(),
            Style::default().fg(Color::DarkGray),
        ));
    }    frame.render_widget(
        Paragraph::new(info_lines)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .border_type(BorderType::Rounded),
            ),
        chunks[0],
    );

    if *asking_another {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("Select another range? "),
                Span::styled("(y/n)", Style::default().fg(accent)),
            ]))
            .alignment(Alignment::Center),
            chunks[1],
        );
        return;
    }

    frame.render_widget(
        input_line(
            format!("Start ayah (1-{}, 'all' for full surah):", info.total_verses),
            start,
            focus == Focus::Start,
            accent,
        ),
        chunks[1],
    );
    frame.render_widget(
        input_line(
            "End ayah (default: full surah):".to_string(),
            end,
            focus == Focus::End,
            accent,
        ),
        chunks[2],
    );
    if let Some(error) = error {
        frame.render_widget(
            Paragraph::new(Line::from(error)).style(Style::default().fg(Color::Red)),
            chunks[3],
        );
    }
}

fn render_reading(
    frame: &mut Frame,
    state: &ReaderState,
    data: &DataHandler,
    config: &ReadingConfig,
    accent: Color,
) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let (pages, page) = match &state.phase {
        ReaderPhase::Reading { pages, page } => (pages, *page),
        _ => return,
    };
    let header = format!(
        "Surah {}: {} ({})    Page {}/{}",
        state.surah,
        state.info.surah_name,
        state.info.surah_name_ar,
        page + 1,
        pages.len()
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            header,
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )])),
        chunks[0],
    );

    let mut ayah_lines: Vec<Line> = Vec::new();
    for ayah in &pages[page] {
        ayah_lines.push(Line::from(vec![
            Span::styled(
                format!("[{}] ", ayah.number),
                Style::default().fg(accent),
            ),
            Span::raw(data.display_content(&ayah.content)),
        ]));
        if config.show_transliteration && !ayah.transliteration.is_empty() {
            ayah_lines.push(Line::styled(
                format!("     {}", ayah.transliteration),
                Style::default().fg(Color::DarkGray),
            ));
        }
        if config.show_english && !ayah.text.is_empty() {
            ayah_lines.push(Line::styled(
                format!("     {}", ayah.text),
                Style::default().fg(Color::Gray),
            ));
        }
        if config.show_turkish && !ayah.translation_tr.is_empty() {
            ayah_lines.push(Line::styled(
                format!("     {}", ayah.translation_tr),
                Style::default().fg(Color::Gray),
            ));
        }
        ayah_lines.push(Line::default());
    }
    frame.render_widget(
        Paragraph::new(ayah_lines)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .border_type(BorderType::Rounded),
            ),
        chunks[1],
    );

    let footer: Vec<Span> = vec![
        Span::styled("n/Enter", Style::default().fg(accent)),
        Span::raw(" next  ·  "),
        Span::styled("p", Style::default().fg(accent)),
        Span::raw(" prev  ·  "),
        Span::styled("a", Style::default().fg(accent)),
        Span::raw(" audio  ·  "),
        Span::styled("k", Style::default().fg(accent)),
        Span::raw(" Ayatul Kursi  ·  "),
        Span::styled("reverse", Style::default().fg(accent)),
        Span::raw(" toggle  ·  "),
        Span::styled("bookmark", Style::default().fg(accent)),
        Span::raw("  ·  "),
        Span::styled("q", Style::default().fg(accent)),
        Span::raw(" range"),
    ];
    let footer_line = if state.command_buffer.is_empty() {
        if let Some(status) = &state.audio_status {
            Line::from(vec![Span::styled(
                status.clone(),
                Style::default().fg(Color::Cyan),
            )])
        } else {
            Line::from(footer)
        }
    } else {
        Line::from(vec![
            Span::styled(
                format!("> {}", state.command_buffer),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("█", Style::default().fg(Color::Gray)),
        ])
    };
    frame.render_widget(Paragraph::new(footer_line), chunks[2]);
}

pub enum ReaderAction {
    None,
    Notice(String),
    BackToMenu,
    ToggleAudio,
    PlayAyatulKursi,
    StopAudio,
    TogglePause,
    SaveBookmark { surah: u32, ayah: u32, note: String },
}

pub fn handle_reader_key(
    state: &mut ReaderState,
    key: KeyEvent,
    data: &mut DataHandler,
) -> ReaderAction {
    match &state.phase {
        ReaderPhase::RangePrompt { .. } => handle_range_key(state, key, data),
        ReaderPhase::Reading { .. } => handle_reading_key(state, key, data),
        ReaderPhase::BookmarkPrompt { .. } => handle_bookmark_key(state, key),
    }
}

fn handle_bookmark_key(state: &mut ReaderState, key: KeyEvent) -> ReaderAction {
    let mut save = None;
    let mut cancel = false;
    if let ReaderPhase::BookmarkPrompt {
        ayahs,
        step,
        ayah_input,
        note_input,
        error,
        saved: _,
    } = &mut state.phase
    {
        match key.code {
            KeyCode::Char(_) | KeyCode::Backspace => match step {
                BookmarkStep::Ayah => ayah_input.handle_key(key),
                BookmarkStep::Note => note_input.handle_key(key),
            },
            KeyCode::Enter => match step {
                BookmarkStep::Ayah => {
                    let ayah = ayah_input.buffer.trim().parse::<u32>().ok();
                    match ayah {
                        Some(n) if ayahs.contains(&n) => {
                            *step = BookmarkStep::Note;
                            *error = None;
                        }
                        _ => {
                            *error = Some(format!(
                                "Invalid ayah number. Choose from: {}",
                                ayahs
                                    .iter()
                                    .map(|n| n.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    }
                }
                BookmarkStep::Note => {
                    let ayah = ayah_input.buffer.trim().parse::<u32>().ok();
                    if let Some(ayah) = ayah {
                        let note: String = note_input
                            .buffer
                            .chars()
                            .take(300)
                            .collect();
                        save = Some((ayah, note));
                    }
                }
            },
            KeyCode::Esc => cancel = true,
            _ => {}
        }
    }
    if let Some((ayah, note)) = save {
        return ReaderAction::SaveBookmark {
            surah: state.surah,
            ayah,
            note,
        };
    }
    if cancel
        && let ReaderPhase::BookmarkPrompt { saved, .. } = &state.phase
    {
        let (pages, page) = saved.clone();
        state.phase = ReaderPhase::Reading { pages, page };
    }
    ReaderAction::None
}

fn handle_range_key(
    state: &mut ReaderState,
    key: KeyEvent,
    data: &mut DataHandler,
) -> ReaderAction {
    let mut begin = false;
    if let ReaderPhase::RangePrompt {
        asking_another,
        focus,
        start,
        end,
        error,
    } = &mut state.phase
    {
            if *asking_another {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        *asking_another = false;
                        start.clear();
                        end.clear();
                        *error = None;
                    }
                    KeyCode::Char('n') => return ReaderAction::BackToMenu,
                    _ => {}
                }
                return ReaderAction::None;
            }

            match key.code {
                KeyCode::Char(_) | KeyCode::Backspace => {
                    match focus {
                        Focus::Start => start,
                        Focus::End => end,
                    }
                    .handle_key(key);
                }
                KeyCode::Enter => {
                    if *focus == Focus::Start {
                        *focus = Focus::End;
                    } else {
                        begin = true;
                    }
                }
                KeyCode::Esc => {
                    *focus = Focus::Start;
                    start.clear();
                    end.clear();
                }
                _ => {}
            }
    }
    if begin {
        state.begin_reading(data);
    }
    ReaderAction::None
}

fn handle_reading_key(
    state: &mut ReaderState,
    key: KeyEvent,
    data: &mut DataHandler,
) -> ReaderAction {
    let mut next_page = false;
    let mut back = false;
    match &state.phase {
        ReaderPhase::Reading { .. } => {}
        _ => return ReaderAction::None,
    }
    if !state.command_buffer.is_empty() {
        return handle_command_typing(state, key, data);
    }
    match key.code {
        KeyCode::Enter => next_page = true,
        KeyCode::Char('n') => next_page = true,
        KeyCode::Char('p') => {
            state.prev_page();
        }
        KeyCode::Char('q') => back = true,
        KeyCode::Char('a') => {
            state.clear_command();
            return ReaderAction::ToggleAudio;
        }
        KeyCode::Char('k') => {
            state.clear_command();
            return if state.surah == 2 {
                ReaderAction::PlayAyatulKursi
            } else {
                ReaderAction::Notice("Ayatul Kursi is verse 255 of Surah 2 (Al-Baqara)".into())
            };
        }
        KeyCode::Char(' ') => {
            state.clear_command();
            return ReaderAction::TogglePause;
        }
        KeyCode::Char('s') => {
            state.clear_command();
            return ReaderAction::StopAudio;
        }
        KeyCode::Char(c) => state.push_command_char(c),
        KeyCode::Backspace => {
            state.command_buffer.pop();
        }
        KeyCode::Esc => state.clear_command(),
        _ => {}
    }
    if next_page && state.next_page() {
        back = true;
    }
    if back {
        state.to_another_range();
    }
    ReaderAction::None
}

fn handle_command_typing(
    state: &mut ReaderState,
    key: KeyEvent,
    data: &mut DataHandler,
) -> ReaderAction {
    match key.code {
        KeyCode::Char(c) => {
            state.push_command_char(c);
            ReaderAction::None
        }
        KeyCode::Backspace => {
            state.command_buffer.pop();
            ReaderAction::None
        }
        KeyCode::Esc => {
            state.clear_command();
            ReaderAction::None
        }
        KeyCode::Enter => {
            let command = std::mem::take(&mut state.command_buffer);
            match command.as_str() {
                "reverse" | "rev" => {
                    data.arabic_reversed = !data.arabic_reversed;
                    ReaderAction::None
                }
                "bookmark" | "bm" => {
                    state.begin_bookmark();
                    ReaderAction::None
                }
                _ => ReaderAction::Notice(format!("Unknown command: {command}")),
            }
        }
        _ => ReaderAction::None,
    }
}