use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::{Frame, Terminal};
use tokio::sync::mpsc;

use crate::api::{DownloadProgress, QuranApiClient};
use crate::audio::{self, AudioManager};
use crate::cache::{DatabaseFiles, QuranCache};
use crate::data::{DataHandler, LUHAIDAN_KEY};
use crate::models::ReciterAudio;
use crate::fuzzy;
use crate::models::CachedSurah;
use crate::paths::Paths;
use crate::settings::Settings;
use crate::srt;
use crate::ui;
use crate::ui::bookmarks::{self, BookmarksState};
use crate::ui::main_menu::MainMenuState;
use crate::ui::reader::{self, ReaderState};
use crate::ui::subtitles::{self, SubtitlePhase, SubtitlesState};
use crate::ui::surah_list::{self, SurahListState};
use crate::ui::splash;

pub enum Screen {
    Bootstrap,
    MainMenu(MainMenuState),
    SurahList(SurahListState),
    Reader(Box<ReaderState>),
    Bookmarks(BookmarksState),
    Subtitles(SubtitlesState),
}

enum BootstrapEvent {
    SurahsDone(HashMap<u32, CachedSurah>),
    DbStarted(String),
    DbFinished(String),
    DbFailed(String),
    AllDone,
}

struct BootstrapState {
    rx: Option<tokio::sync::mpsc::Receiver<BootstrapEvent>>,
    progress_rx: Option<tokio::sync::mpsc::Receiver<DownloadProgress>>,
    tx: tokio::sync::mpsc::Sender<BootstrapEvent>,
    started: bool,
    surah_total: u32,
    surah_done: u32,
    surah_failed: u32,
    db_total: usize,
    db_done: usize,
    current: String,
    phase: &'static str,
    fatal: Option<String>,
    finished: bool,
}

impl BootstrapState {
    fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        Self {
            rx: Some(rx),
            progress_rx: None,
            tx,
            started: false,
            surah_total: 0,
            surah_done: 0,
            surah_failed: 0,
            db_total: 0,
            db_done: 0,
            current: String::new(),
            phase: "Surahs",
            fatal: None,
            finished: false,
        }
    }

    fn splash_data(&self) -> splash::SplashData {
        splash::SplashData {
            phase: self.phase,
            current: self.current.clone(),
            done: if self.phase == "Surahs" {
                self.surah_done
            } else {
                self.db_done as u32
            },
            total: if self.phase == "Surahs" {
                self.surah_total
            } else {
                self.db_total as u32
            },
            failed: self.surah_failed,
            fatal: self.fatal.clone(),
        }
    }
}

enum ScreenAction {
    None,
    Quit,
    OpenSurah(u32),
    ShowSurahList,
    BackToMainMenu,
    Notice(String),
    ToggleAudio,
    PlayAyatulKursi,
    StopAudio,
    TogglePause,
    ShowBookmarks,
    SaveBookmark { surah: u32, ayah: u32, note: String },
    AddBookmark(String),
    DeleteBookmark(usize),
    ShowSubtitles,
    GenerateSrt { surah: u32, start: u32, end: u32 },
}

struct AudioResult {
    path: PathBuf,
    surah: u32,
    reciter: String,
    is_ayatul_kursi: bool,
    outcome: Result<(), String>,
}

pub struct App {
    pub settings: Settings,
    pub paths: Paths,
    pub client: QuranApiClient,
    cache: QuranCache,
    db_files: DatabaseFiles,
    data: Option<DataHandler>,
    screen: Screen,
    bootstrap: BootstrapState,
    accent: Color,
    pub should_quit: bool,
    notice: Option<String>,
    audio: AudioManager,
    audio_pending: Option<mpsc::Receiver<AudioResult>>,
}

impl App {
    pub fn new(
        settings: Settings,
        paths: Paths,
        client: QuranApiClient,
        cache: QuranCache,
        db_files: DatabaseFiles,
        data: Option<DataHandler>,
    ) -> Self {
        let accent = ui::theme_color(&settings.theme_color);
        let screen = match &data {
            Some(_) => Screen::MainMenu(MainMenuState::default()),
            None => Screen::Bootstrap,
        };
        Self {
            settings,
            paths,
            client,
            cache,
            db_files,
            data,
            screen,
            bootstrap: BootstrapState::new(),
            accent,
            should_quit: false,
            notice: None,
            audio: AudioManager::new(),
            audio_pending: None,
        }
    }

    fn start_bootstrap(&mut self) {
        let missing = self.cache.missing_surahs();
        if missing.is_empty() {
            self.start_db_downloads();
            return;
        }
        self.bootstrap.surah_total = missing.len() as u32;
        self.bootstrap.phase = "Surahs";
        let client = self.client.clone();
        let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(64);
        self.bootstrap.progress_rx = Some(progress_rx);
        let tx = self.bootstrap.tx.clone();
        tokio::spawn(async move {
            let downloaded = client.download_missing(missing, progress_tx).await;
            let _ = tx.send(BootstrapEvent::SurahsDone(downloaded)).await;
        });
    }

    fn start_db_downloads(&mut self) {
        let missing = self.db_files.missing();
        if missing.is_empty() {
            self.bootstrap.finished = true;
            return;
        }
        self.bootstrap.db_total = missing.len();
        self.bootstrap.phase = "Databases";
        self.bootstrap.current = "Downloading translation databases…".into();
        let client = self.client.clone();
        let tx = self.bootstrap.tx.clone();
        let files = missing
            .iter()
            .map(|(url, path)| (url.to_string(), (*path).clone()))
            .collect::<Vec<_>>();
        tokio::spawn(async move {
            for (url, path) in files {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let _ = tx.send(BootstrapEvent::DbStarted(name.clone())).await;
                let result = async {
                    let response = client
                        .client()
                        .get(&url)
                        .send()
                        .await
                        .with_context(|| format!("failed to download {name}"))?;
                    let bytes = response.error_for_status()?.bytes().await?;
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir)?;
                    }
                    std::fs::write(&path, bytes)?;
                    Ok::<_, anyhow::Error>(())
                }
                .await;
                if result.is_ok() {
                    let _ = tx.send(BootstrapEvent::DbFinished(name)).await;
                } else {
                    let _ = tx.send(BootstrapEvent::DbFailed(name)).await;
                }
            }
            let _ = tx.send(BootstrapEvent::AllDone).await;
        });
    }

    async fn drain_bootstrap(&mut self) {
        let Some(mut progress_rx) = self.bootstrap.progress_rx.take() else {
            return;
        };
        while let Ok(progress) = progress_rx.try_recv() {
            match progress {
                DownloadProgress::Started(n) => {
                    self.bootstrap.current = format!("Fetching surah {n}…");
                }
                DownloadProgress::Finished(n) => {
                    self.bootstrap.surah_done += 1;
                    self.bootstrap.current = format!("Fetched surah {n}");
                }
                DownloadProgress::Failed(n) => {
                    self.bootstrap.surah_failed += 1;
                    self.bootstrap.current = format!("Failed surah {n}");
                }
                DownloadProgress::Done => {}
            }
        }
        self.bootstrap.progress_rx = Some(progress_rx);

        let Some(mut rx) = self.bootstrap.rx.take() else {
            return;
        };
        while let Ok(event) = rx.try_recv() {
            match event {
                BootstrapEvent::SurahsDone(downloaded) => {
                    for (n, surah) in downloaded {
                        self.cache.insert(n, surah);
                    }
                    if let Err(error) = self.cache.save() {
                        self.bootstrap.fatal = Some(format!("Failed to save cache: {error}"));
                        self.bootstrap.rx = Some(rx);
                        return;
                    }
                    self.start_db_downloads();
                }
                BootstrapEvent::DbStarted(name) => {
                    self.bootstrap.current = format!("Downloading {name}…");
                }
                BootstrapEvent::DbFinished(name) => {
                    self.bootstrap.db_done += 1;
                    self.bootstrap.current = format!("Downloaded {name}");
                }
                BootstrapEvent::DbFailed(name) => {
                    self.bootstrap.fatal = Some(format!(
                        "Failed to download {name}. Check your connection and retry."
                    ));
                    self.bootstrap.rx = Some(rx);
                    return;
                }
                BootstrapEvent::AllDone => {
                    self.bootstrap.finished = true;
                }
            }
        }
        if self.bootstrap.finished && self.bootstrap.fatal.is_none() {
            let db_files = self.db_files.clone();
            match DataHandler::load(self.cache.clone(), &db_files) {
                Ok(data) => {
                    self.data = Some(data);
                    self.bootstrap.rx = None;
                    self.screen = Screen::MainMenu(MainMenuState::default());
                    return;
                }
                Err(error) => {
                    self.bootstrap.fatal = Some(format!("Failed to load data: {error}"));
                }
            }
        }
        self.bootstrap.rx = Some(rx);
    }

    pub async fn tick(&mut self) {
        if matches!(self.screen, Screen::Bootstrap) {
            if !self.bootstrap.started && self.bootstrap.fatal.is_none() {
                self.bootstrap.started = true;
                self.start_bootstrap();
            }
            self.drain_bootstrap().await;
        }
        self.drain_audio();
    }

    fn drain_audio(&mut self) {
        if let Some(mut rx) = self.audio_pending.take() {
            while let Ok(result) = rx.try_recv() {
                match result.outcome {
                    Ok(()) => {
                        self.audio
                            .play_file(&result.path, result.surah, result.reciter, result.is_ayatul_kursi)
                            .map_err(|e| self.notice = Some(format!("Playback failed: {e}")))
                            .ok();
                    }
                    Err(error) => {
                        self.notice = Some(format!("Audio download failed: {error}"));
                    }
                }
            }
            if !rx.is_closed() {
                self.audio_pending = Some(rx);
            }
        }
        if let Screen::Reader(state) = &mut self.screen
            && matches!(state.phase, reader::ReaderPhase::Reading { .. })
        {
            state.audio_status = self.audio.status_line();
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        match &mut self.screen {
            Screen::Bootstrap => {
                let splash_data = self.bootstrap.splash_data();
                splash::render(frame, &splash_data, self.accent);
            }
            Screen::MainMenu(state) => ui::main_menu::render(frame, state, self.accent),
            Screen::SurahList(state) => {
                let names = self
                    .data
                    .as_ref()
                    .map(|d| d.get_all_surah_names())
                    .unwrap_or_default();
                surah_list::render(frame, state, &names, self.accent);
            }
            Screen::Reader(state) => {
                let data = self.data.as_ref().expect("data loaded before reader");
                let config = self.settings.reading_config.clone();
                reader::render(frame, state, data, &config, self.accent);
            }
            Screen::Bookmarks(state) => {
                let names = self
                    .data
                    .as_ref()
                    .map(|d| d.get_all_surah_names())
                    .unwrap_or_default();
                bookmarks::render(frame, state, &names, self.accent);
            }
            Screen::Subtitles(state) => {
                let data = self.data.as_ref().expect("data loaded before subtitles");
                let config = self.settings.subtitle_config.clone();
                subtitles::render(frame, state, data, &config, self.accent);
            }
        }
        if let Some(notice) = &self.notice {
            let area = centered_rect(60, 15, frame.area());
            frame.render_widget(
                Paragraph::new(Line::from(notice.as_str()))
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(self.accent))
                            .border_type(BorderType::Rounded),
                    ),
                area,
            );
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Char('c'))
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.should_quit = true;
            return;
        }
        if self.notice.take().is_some() {
            return;
        }
        if matches!(self.screen, Screen::Bootstrap) {
            if self.bootstrap.fatal.is_some() && matches!(key.code, KeyCode::Char('q')) {
                self.should_quit = true;
            }
            return;
        }

        let action = match &mut self.screen {
            Screen::MainMenu(state) => Self::handle_main_menu_key(&mut self.data, state, key),
            Screen::SurahList(state) => Self::handle_surah_list_key(&self.data, state, key),
            Screen::Reader(state) => {
                let Some(data) = self.data.as_mut() else {
                    return;
                };
                match reader::handle_reader_key(state, key, data) {
                    reader::ReaderAction::None => ScreenAction::None,
                    reader::ReaderAction::Notice(message) => ScreenAction::Notice(message),
                    reader::ReaderAction::BackToMenu => ScreenAction::BackToMainMenu,
                    reader::ReaderAction::ToggleAudio => ScreenAction::ToggleAudio,
                    reader::ReaderAction::PlayAyatulKursi => ScreenAction::PlayAyatulKursi,
                    reader::ReaderAction::StopAudio => ScreenAction::StopAudio,
                    reader::ReaderAction::TogglePause => ScreenAction::TogglePause,
                    reader::ReaderAction::SaveBookmark { surah, ayah, note } => {
                        ScreenAction::SaveBookmark { surah, ayah, note }
                    }
                }
            }
            Screen::Bookmarks(state) => match bookmarks::handle_key(state, key) {
                bookmarks::BookmarksAction::None => ScreenAction::None,
                bookmarks::BookmarksAction::Back => ScreenAction::BackToMainMenu,
                bookmarks::BookmarksAction::Add(input) => ScreenAction::AddBookmark(input),
                bookmarks::BookmarksAction::Delete(index) => ScreenAction::DeleteBookmark(index),
            },
            Screen::Subtitles(state) => {
                let Some(data) = self.data.as_mut() else {
                    return;
                };
                match subtitles::handle_key(state, data, key) {
                    subtitles::SubtitlesAction::None => ScreenAction::None,
                    subtitles::SubtitlesAction::Back => ScreenAction::BackToMainMenu,
                    subtitles::SubtitlesAction::SetSurah(n) => {
                        if let Some(info) = data.get_surah_info(n) {
                            state.set_surah(n, info.surah_name.clone(), info.total_verses);
                        } else {
                            state.error = Some("Surah data is not available yet".into());
                        }
                        ScreenAction::None
                    }
                    subtitles::SubtitlesAction::SetRange => {
                        let start = state.start.buffer.trim().parse::<u32>();
                        let end = state.end.buffer.trim().parse::<u32>();
                        match (start, end) {
                            (Ok(s), Ok(e))
                                if s >= 1
                                    && s <= e
                                    && e <= state.total_verses =>
                            {
                                state.error = None;
                                state.phase = SubtitlePhase::Confirm;
                            }
                            _ => {
                                state.error = Some(format!(
                                    "Invalid range: start 1-{}, end >= start and <= {}",
                                    state.total_verses, state.total_verses
                                ));
                            }
                        }
                        ScreenAction::None
                    }
                    subtitles::SubtitlesAction::Generate => {
                        let start = state.start.buffer.trim().parse::<u32>().unwrap_or(1);
                        let end = state.end.buffer.trim().parse::<u32>().unwrap_or(1);
                        ScreenAction::GenerateSrt {
                            surah: state.surah_number,
                            start,
                            end,
                        }
                    }
                    subtitles::SubtitlesAction::ToggleTransliteration => {
                        self.settings.subtitle_config.include_transliteration =
                            !self.settings.subtitle_config.include_transliteration;
                        self.save_settings();
                        ScreenAction::None
                    }
                    subtitles::SubtitlesAction::ToggleTurkish => {
                        self.settings.subtitle_config.include_turkish =
                            !self.settings.subtitle_config.include_turkish;
                        self.save_settings();
                        ScreenAction::None
                    }
                    subtitles::SubtitlesAction::ToggleEnglish => {
                        self.settings.subtitle_config.include_english =
                            !self.settings.subtitle_config.include_english;
                        self.save_settings();
                        ScreenAction::None
                    }
                }
            },
            Screen::Bootstrap => ScreenAction::None,
        };
        self.apply_action(action);
    }

    fn apply_action(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Quit => self.should_quit = true,
            ScreenAction::OpenSurah(n) => self.open_surah(n),
            ScreenAction::ShowSurahList => {
                self.screen = Screen::SurahList(SurahListState::default());
            }
            ScreenAction::BackToMainMenu => {
                self.audio.stop();
                self.screen = Screen::MainMenu(MainMenuState::default());
            }
            ScreenAction::Notice(message) => self.notice = Some(message),
            ScreenAction::ToggleAudio => self.toggle_surah_audio(),
            ScreenAction::PlayAyatulKursi => self.play_ayatul_kursi(),
            ScreenAction::StopAudio => self.audio.stop(),
            ScreenAction::TogglePause => self.audio.toggle_pause(),
            ScreenAction::ShowBookmarks => self.show_bookmarks(),
            ScreenAction::SaveBookmark { surah, ayah, note } => {
                self.settings.add_bookmark(surah, ayah, note);
                self.save_settings();
                if let Screen::Reader(state) = &mut self.screen {
                    state.finish_bookmark();
                }
                self.notice = Some(format!("Bookmark saved for ayah {ayah} of surah {surah}"));
            }
            ScreenAction::AddBookmark(input) => {
                let action = self.add_bookmark_from_input(input);
                self.apply_action(action);
            }
            ScreenAction::DeleteBookmark(index) => self.delete_bookmark(index),
            ScreenAction::ShowSubtitles => self.show_subtitles(),
            ScreenAction::GenerateSrt { surah, start, end } => self.generate_srt(surah, start, end),
        }
    }

    fn save_settings(&mut self) {
        if let Err(error) = self.settings.save(&self.paths.settings_file) {
            self.notice = Some(format!("Failed to save settings: {error}"));
        }
    }

    fn show_bookmarks(&mut self) {
        let mut state = BookmarksState::default();
        state.refresh(&self.settings.bookmarks);
        self.screen = Screen::Bookmarks(state);
    }

    fn show_subtitles(&mut self) {
        self.audio.stop();
        self.screen = Screen::Subtitles(SubtitlesState::default());
    }

    fn generate_srt(&mut self, surah: u32, start: u32, end: u32) {
        let Some(data) = self.data.as_ref() else {
            self.notice = Some("Data not loaded yet".into());
            return;
        };
        let surah_name = data
            .get_surah_info(surah)
            .map(|info| info.surah_name.clone())
            .unwrap_or_else(|| format!("Surah{surah:03}"));
        match srt::generate_srt_file(
            data,
            &self.paths.subtitles_dir,
            surah,
            &surah_name,
            start,
            end,
            &self.settings.subtitle_config,
        ) {
            Ok(path) => {
                if let Screen::Subtitles(state) = &mut self.screen {
                    state.phase = SubtitlePhase::Surah;
                    state.surah.clear();
                    state.error = Some(format!("Saved: {}", path.display()));
                }
            }
            Err(error) => {
                if let Screen::Subtitles(state) = &mut self.screen {
                    state.error = Some(error.to_string());
                }
            }
        }
    }

    fn add_bookmark_from_input(&mut self, input: String) -> ScreenAction {
        match bookmarks::parse_add_input(&input) {
            Ok((surah, ayah, note)) => {
                if let Some(info) = self.data.as_ref().and_then(|d| d.get_surah_info(surah))
                    && (ayah == 0 || ayah > info.total_verses)
                {
                    return ScreenAction::Notice(format!(
                        "Ayah {ayah} is out of range for surah {surah} (1-{})",
                        info.total_verses
                    ));
                }
                self.settings.add_bookmark(surah, ayah, note);
                self.save_settings();
                if let Screen::Bookmarks(state) = &mut self.screen {
                    state.refresh(&self.settings.bookmarks);
                }
                ScreenAction::Notice(format!("Bookmark saved for ayah {ayah} of surah {surah}"))
            }
            Err(message) => ScreenAction::Notice(message),
        }
    }

    fn delete_bookmark(&mut self, index: usize) {
        let target = if let Screen::Bookmarks(state) = &self.screen {
            state.entries.get(index).map(|(s, b)| (*s, b.ayah))
        } else {
            None
        };
        if let Some((surah, ayah)) = target {
            self.settings.remove_bookmark(surah, Some(ayah));
            self.save_settings();
            if let Screen::Bookmarks(state) = &mut self.screen {
                state.refresh(&self.settings.bookmarks);
            }
        }
    }

    fn handle_main_menu_key(
        data: &mut Option<DataHandler>,
        state: &mut MainMenuState,
        key: KeyEvent,
    ) -> ScreenAction {
        match key.code {
            KeyCode::Char(_) | KeyCode::Backspace => {
                state.input.handle_key(key);
                ScreenAction::None
            }
            KeyCode::Enter => Self::submit_main_menu(data, state),
            KeyCode::Esc => {
                state.input.clear();
                state.did_you_mean = None;
                state.notice = None;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn submit_main_menu(
        data: &mut Option<DataHandler>,
        state: &mut MainMenuState,
    ) -> ScreenAction {
        let input = std::mem::take(&mut state.input.buffer);
        let input = input.trim().to_string();
        if input.is_empty() {
            return ScreenAction::None;
        }

        if let Some(suggestions) = &state.did_you_mean {
            if input == "r" {
                state.did_you_mean = None;
                return ScreenAction::None;
            }
            if let Ok(index) = input.parse::<usize>()
            && (1..=suggestions.len()).contains(&index)
        {
            let name = suggestions[index - 1].clone();
            state.did_you_mean = None;
            return Self::surah_number_by_name(data, &name)
                .map(ScreenAction::OpenSurah)
                .unwrap_or(ScreenAction::None);
        }
            state.did_you_mean = None;
            return ScreenAction::None;
        }

        match input.as_str() {
            "quit" | "exit" | "q" => ScreenAction::Quit,
            "list" | "ls" => ScreenAction::ShowSurahList,
            "bookmarks" | "bm" => ScreenAction::ShowBookmarks,
            "sub" => ScreenAction::ShowSubtitles,
            "settings" | "st" | "audio" | "au" | "info" | "i" | "readme" | "rd" => {
                state.notice = Some("That feature arrives in a later build".into());
                ScreenAction::None
            }
            _ => {
                if let Ok(n) = input.parse::<u32>() {
                    if (1..=114).contains(&n) {
                        return ScreenAction::OpenSurah(n);
                    }
                    state.notice = Some("Invalid surah number (must be 1-114)".into());
                    return ScreenAction::None;
                }
                let names = data
                    .as_ref()
                    .map(|d| d.get_all_surah_names())
                    .unwrap_or_default();
                let matches = fuzzy::get_close_matches(&input, &names, 5, 0.5);
                match matches.len() {
                    0 => {
                        state.notice = Some("No matching surah found".into());
                        ScreenAction::None
                    }
                    1 => Self::surah_number_by_name(data, matches[0])
                        .map(ScreenAction::OpenSurah)
                        .unwrap_or(ScreenAction::None),
                    _ => {
                        state.did_you_mean = Some(matches.into_iter().map(String::from).collect());
                        ScreenAction::None
                    }
                }
            }
        }
    }

    fn toggle_surah_audio(&mut self) {
        if self.audio.has_playback() {
            self.audio.stop();
            return;
        }
        let surah = match &self.screen {
            Screen::Reader(state) => state.surah,
            _ => return,
        };
        self.play_surah_audio(surah);
    }

    fn play_surah_audio(&mut self, surah: u32) {
        if !self.audio.available() {
            self.notice = Some("No audio output device is available on this system".into());
            return;
        }
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let Some(info) = data.get_surah_info(surah) else {
            return;
        };
        let (reciter, url) = Self::resolve_reciter(&self.settings, &info.audio, surah);
        let path = audio::surah_audio_path(&self.paths.audio_cache_dir, surah, &reciter);
        self.start_audio(path, surah, reciter, false, url);
    }

    fn play_ayatul_kursi(&mut self) {
        if !self.audio.available() {
            self.notice = Some("No audio output device is available on this system".into());
            return;
        }
        let (reciter, url) = match &self.settings.ayatul_kursi {
            Some(pref) => (pref.reciter_name.clone(), pref.reciter_url.clone()),
            None => {
                let (name, url) = audio::AYATUL_KURSI_RECITERS[0];
                (name.to_string(), url.to_string())
            }
        };
        let path = audio::ayatul_kursi_audio_path(&self.paths.audio_cache_dir, &reciter);
        self.start_audio(path, 2, reciter, true, url);
    }

    fn start_audio(
        &mut self,
        path: PathBuf,
        surah: u32,
        reciter: String,
        is_ayatul_kursi: bool,
        url: String,
    ) {
        if path.exists() {
            match self
                .audio
                .play_file(&path, surah, reciter, is_ayatul_kursi)
            {
                Ok(()) => {}
                Err(error) => self.notice = Some(format!("Playback failed: {error}")),
            }
            return;
        }
        if self.audio_pending.is_some() {
            self.notice = Some("Audio is already downloading".into());
            return;
        }
        let label = if is_ayatul_kursi {
            "Ayatul Kursi".to_string()
        } else {
            format!("surah {surah}")
        };
        self.notice = Some(format!("Downloading audio for {label}…"));
        let client = self.client.clone();
        let (tx, rx) = mpsc::channel(1);
        self.audio_pending = Some(rx);
        tokio::spawn(async move {
            let outcome = audio::download_audio(&client, &url, &path, None)
                .await
                .map_err(|error| error.to_string());
            let _ = tx
                .send(AudioResult {
                    path,
                    surah,
                    reciter,
                    is_ayatul_kursi,
                    outcome,
                })
                .await;
        });
    }

    fn resolve_reciter(
        settings: &Settings,
        audio: &HashMap<String, ReciterAudio>,
        surah: u32,
    ) -> (String, String) {
    if let Some(pref) = settings.surah_reciter(surah) {
        return (pref.reciter_name.clone(), pref.reciter_url.clone());
    }
    if let Some(entry) = audio.get(LUHAIDAN_KEY) {
            return (entry.reciter.clone(), entry.url.clone());
        }
        if let Some((_, entry)) = audio.iter().next() {
            return (entry.reciter.clone(), entry.url.clone());
        }
        (
            crate::data::LUHAIDAN_NAME.to_string(),
            crate::data::LUHAIDAN_URL.to_string(),
        )
    }

fn surah_number_by_name(data: &Option<DataHandler>, name: &str) -> Option<u32> {
        let names = data
            .as_ref()
            .map(|d| d.get_all_surah_names())
            .unwrap_or_default();
        names
            .iter()
            .position(|n| n == name)
            .map(|i| i as u32 + 1)
    }

    fn open_surah(&mut self, n: u32) {
        let Some(data) = self.data.as_mut() else {
            return;
        };
        let Some(info) = data.get_surah_info(n) else {
            if let Screen::MainMenu(state) = &mut self.screen {
                state.notice = Some(format!("Surah {n} is not available yet"));
            }
            return;
        };
        if self.audio.current.as_ref().is_some_and(|p| p.surah != n) {
            self.audio.stop();
        }
        self.screen = Screen::Reader(Box::new(ReaderState::new(n, info)));
    }

    fn handle_surah_list_key(
        data: &Option<DataHandler>,
        state: &mut SurahListState,
        key: KeyEvent,
    ) -> ScreenAction {
        let names_len = data
            .as_ref()
            .map(|d| d.get_all_surah_names().len())
            .unwrap_or(0);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                state.selected = state.selected.saturating_sub(surah_list::COLUMNS);
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected + surah_list::COLUMNS < names_len {
                    state.selected += surah_list::COLUMNS;
                }
                ScreenAction::None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                state.selected = state.selected.saturating_sub(1);
                ScreenAction::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if state.selected + 1 < names_len {
                    state.selected += 1;
                }
                ScreenAction::None
            }
            KeyCode::Enter => ScreenAction::OpenSurah(state.selected as u32 + 1),
            KeyCode::Char('q') | KeyCode::Esc => ScreenAction::BackToMainMenu,
            _ => ScreenAction::None,
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let chunks = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(chunks[1])[1]
}

pub async fn run(mut app: App) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal, &mut app).await;
    ratatui::restore();
    result
}

async fn run_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;
        if app.should_quit {
            break;
        }
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key(key);
        }
        app.tick().await;
    }
    Ok(())
}