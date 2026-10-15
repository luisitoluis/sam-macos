// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::images::{Images, Slot};
use super::worker::{self, Job, StatValue, UiEvent};
use crate::backend::app_lister::{AppModel, AppModelType};
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::text::{self, Id, Language, Message};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::HashMap;
use std::io::{self, stdout};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdateCheck {
    Idle,
    Checking,
    Current,
    Available,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    Aura,
    Ayu,
    Carbonfox,
    Catppuccin,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    Cobalt2,
    Cursor,
    Dracula,
    Everforest,
    Flexoki,
    Github,
    Gruvbox,
    Kanagawa,
    LucentOrng,
    Material,
    Matrix,
    Mercury,
    Monokai,
    Nightowl,
    Nord,
    OneDark,
    Opencode,
    Orng,
    OsakaJade,
    Palenight,
    Rosepine,
    Solarized,
    Synthwave84,
    Tokyonight,
    Vercel,
    Vesper,
    Zenburn,
}

impl Theme {
    const ALL: [Theme; 33] = [
        Theme::Aura,
        Theme::Ayu,
        Theme::Carbonfox,
        Theme::Catppuccin,
        Theme::CatppuccinFrappe,
        Theme::CatppuccinMacchiato,
        Theme::Cobalt2,
        Theme::Cursor,
        Theme::Dracula,
        Theme::Everforest,
        Theme::Flexoki,
        Theme::Github,
        Theme::Gruvbox,
        Theme::Kanagawa,
        Theme::LucentOrng,
        Theme::Material,
        Theme::Matrix,
        Theme::Mercury,
        Theme::Monokai,
        Theme::Nightowl,
        Theme::Nord,
        Theme::OneDark,
        Theme::Opencode,
        Theme::Orng,
        Theme::OsakaJade,
        Theme::Palenight,
        Theme::Rosepine,
        Theme::Solarized,
        Theme::Synthwave84,
        Theme::Tokyonight,
        Theme::Vercel,
        Theme::Vesper,
        Theme::Zenburn,
    ];

    fn name(self) -> &'static str {
        match self {
            Theme::Aura => "aura",
            Theme::Ayu => "ayu",
            Theme::Carbonfox => "carbonfox",
            Theme::Catppuccin => "catppuccin",
            Theme::CatppuccinFrappe => "catppuccin-frappe",
            Theme::CatppuccinMacchiato => "catppuccin-macchiato",
            Theme::Cobalt2 => "cobalt2",
            Theme::Cursor => "cursor",
            Theme::Dracula => "dracula",
            Theme::Everforest => "everforest",
            Theme::Flexoki => "flexoki",
            Theme::Github => "github",
            Theme::Gruvbox => "gruvbox",
            Theme::Kanagawa => "kanagawa",
            Theme::LucentOrng => "lucent-orng",
            Theme::Material => "material",
            Theme::Matrix => "matrix",
            Theme::Mercury => "mercury",
            Theme::Monokai => "monokai",
            Theme::Nightowl => "nightowl",
            Theme::Nord => "nord",
            Theme::OneDark => "one-dark",
            Theme::Opencode => "opencode",
            Theme::Orng => "orng",
            Theme::OsakaJade => "osaka-jade",
            Theme::Palenight => "palenight",
            Theme::Rosepine => "rosepine",
            Theme::Solarized => "solarized",
            Theme::Synthwave84 => "synthwave84",
            Theme::Tokyonight => "tokyonight",
            Theme::Vercel => "vercel",
            Theme::Vesper => "vesper",
            Theme::Zenburn => "zenburn",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.name() == name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    Library,
    Game,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameTab {
    Achievements,
    Stats,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Typing {
    Search,
    AchievementFilter,
    StatFilter,
    StatValue,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirm {
    UnlockAll,
    Reset,
}

struct GameView {
    app_id: u32,
    name: String,
    developer: String,
    image_url: Option<String>,
    kind: AppModelType,
    tab: GameTab,
    loading: bool,
    achievements: Vec<AchievementInfo>,
    stats: Vec<StatInfo>,
    drafts: HashMap<String, String>,
    achievement_query: String,
    stat_query: String,
    achievement_index: usize,
    stat_index: usize,
}

struct App {
    jobs: Sender<Job>,
    events: Receiver<UiEvent>,
    worker: Option<JoinHandle<()>>,
    inflight: u32,
    apps: Vec<AppModel>,
    search: String,
    hide_junk: bool,
    library_index: usize,
    game: Option<GameView>,
    pane: Pane,
    typing: Option<Typing>,
    settings: bool,
    settings_index: usize,
    help: bool,
    confirm: Option<Confirm>,
    confirm_accept: bool,
    notice: Option<Result<Message, Message>>,
    language: Language,
    theme: Theme,
    appearance: bool,
    theme_query: String,
    theme_index: usize,
    theme_saved: Theme,
    update_tx: Sender<UpdateCheck>,
    update_rx: Receiver<UpdateCheck>,
    update_check: UpdateCheck,
    images: Images,
    quit: bool,
}

pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    let result = {
        let (jobs, events, worker) = worker::spawn();
        let mut app = App::new(jobs, events, worker);
        app.restore();
        app.send(Job::LoadApps);
        let result = event_loop(&mut terminal, &mut app);
        app.store();
        let _ = app.jobs.send(Job::Shutdown);
        if let Some(worker) = app.worker.take() {
            let _ = worker.join();
        }
        result
    };
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    while !app.quit {
        app.poll();
        terminal.draw(|frame| app.draw(frame))?;
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
    }
    Ok(())
}

impl App {
    fn new(jobs: Sender<Job>, events: Receiver<UiEvent>, worker: JoinHandle<()>) -> Self {
        let (update_tx, update_rx) = mpsc::channel();
        Self {
            jobs,
            events,
            worker: Some(worker),
            inflight: 0,
            apps: Vec::new(),
            search: String::new(),
            hide_junk: true,
            library_index: 0,
            game: None,
            pane: Pane::Library,
            typing: None,
            settings: false,
            settings_index: 0,
            help: false,
            confirm: None,
            confirm_accept: false,
            notice: Some(Ok(Message::Phrase(Id::LoadingLibrary))),
            language: Language::from_locale(),
            theme: Theme::Opencode,
            appearance: false,
            theme_query: String::new(),
            theme_index: Theme::ALL
                .iter()
                .position(|theme| *theme == Theme::Opencode)
                .unwrap_or(0),
            theme_saved: Theme::Opencode,
            update_tx,
            update_rx,
            update_check: UpdateCheck::Idle,
            images: Images::new(),
            quit: false,
        }
    }

    fn send(&mut self, job: Job) {
        self.inflight += 1;
        if self.jobs.send(job).is_err() {
            self.inflight = self.inflight.saturating_sub(1);
            self.notice = Some(Err(Message::Phrase(Id::WorkerStopped)));
        }
    }

    fn poll(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.inflight = self.inflight.saturating_sub(1);
            self.handle(event);
        }
        while let Ok(update) = self.update_rx.try_recv() {
            self.update_check = update;
        }
        self.images.poll();
    }

    fn handle(&mut self, event: UiEvent) {
        match event {
            UiEvent::Apps(result) => match result {
                Ok(mut apps) => {
                    apps.sort_by(|a, b| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()));
                    let count = apps.len();
                    self.apps = apps;
                    self.library_index = 0;
                    self.notice = Some(Ok(Message::Games(count)));
                }
                Err(err) => self.notice = Some(Err(Message::Own(err))),
            },
            UiEvent::Details {
                app_id,
                achievements,
                stats,
                message,
            } => self.apply_details(app_id, achievements, stats, message),
            UiEvent::Achievements {
                app_id,
                achievements,
                message,
            } => {
                if let Some(game) = self.game.as_mut().filter(|game| game.app_id == app_id) {
                    game.loading = false;
                    match achievements {
                        Ok(list) => {
                            game.achievements = list;
                            self.notice = Some(Ok(Message::Phrase(message)));
                        }
                        Err(err) => self.notice = Some(Err(Message::Own(err))),
                    }
                }
            }
            UiEvent::Stats {
                app_id,
                stats,
                message,
            } => {
                if let Some(game) = self.game.as_mut().filter(|game| game.app_id == app_id) {
                    game.loading = false;
                    match stats {
                        Ok(list) => {
                            game.drafts = drafts_from(&list);
                            game.stats = list;
                            self.notice = Some(Ok(Message::Phrase(message)));
                        }
                        Err(err) => self.notice = Some(Err(Message::Own(err))),
                    }
                }
            }
            UiEvent::Notice(result) => {
                if let Some(game) = self.game.as_mut() {
                    game.loading = false;
                }
                self.notice = Some(match result {
                    Ok(text) => Ok(Message::Own(text)),
                    Err(text) => Err(Message::Own(text)),
                });
            }
        }
    }

    fn apply_details(
        &mut self,
        app_id: u32,
        achievements: Result<Vec<AchievementInfo>, String>,
        stats: Result<Vec<StatInfo>, String>,
        message: Option<Id>,
    ) {
        let Some(game) = self.game.as_mut().filter(|game| game.app_id == app_id) else {
            return;
        };
        game.loading = false;
        let mut error = None;
        match achievements {
            Ok(list) => game.achievements = list,
            Err(err) => error = Some(err),
        }
        match stats {
            Ok(list) => {
                game.drafts = drafts_from(&list);
                game.stats = list;
            }
            Err(err) => error = Some(err),
        }
        let name = game.name.clone();
        self.notice = Some(match (error, message) {
            (Some(err), _) => Err(Message::Own(err)),
            (None, Some(message)) => Ok(Message::Phrase(message)),
            (None, None) => Ok(Message::Loaded(name)),
        });
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if self.confirm.is_some() {
            self.on_confirm_key(key);
            return;
        }
        if self.settings {
            self.on_settings_key(key);
            return;
        }
        if self.help {
            self.on_help_key(key);
            return;
        }
        if let Some(typing) = self.typing {
            self.on_typing_key(typing, key);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('s') => self.open_settings(),
            KeyCode::Char('h') if self.pane == Pane::Library => self.hide_junk = !self.hide_junk,
            KeyCode::Char('r') if self.inflight == 0 => self.refresh(),
            KeyCode::Char('/') => {
                self.typing = Some(match self.pane {
                    Pane::Library => Typing::Search,
                    Pane::Game => match self.game.as_ref().map(|game| game.tab) {
                        Some(GameTab::Stats) => Typing::StatFilter,
                        _ => Typing::AchievementFilter,
                    },
                })
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::PageUp => self.move_selection(-8),
            KeyCode::PageDown => self.move_selection(8),
            KeyCode::Home => self.move_selection(isize::MIN),
            KeyCode::End => self.move_selection(isize::MAX),
            KeyCode::Enter => self.activate(),
            KeyCode::Esc | KeyCode::Backspace if self.pane == Pane::Game => self.close_game(),
            KeyCode::Tab | KeyCode::BackTab if self.pane == Pane::Game => self.switch_tab(),
            KeyCode::Char(' ') if self.pane == Pane::Game => self.toggle_selected_achievement(),
            KeyCode::Char('u') if self.pane == Pane::Game => self.ask(Confirm::UnlockAll),
            KeyCode::Char('x') if self.pane == Pane::Game => self.ask(Confirm::Reset),
            KeyCode::Char('l') if self.pane == Pane::Game => self.launch(),
            KeyCode::Left if self.pane == Pane::Game => self.pane = Pane::Library,
            KeyCode::Right if self.game.is_some() => self.pane = Pane::Game,
            _ => {}
        }
    }

    fn ask(&mut self, confirm: Confirm) {
        self.confirm = Some(confirm);
        self.confirm_accept = false;
    }

    fn on_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => self.confirm = None,
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.confirm_accept = !self.confirm_accept;
            }
            KeyCode::Char('y') => self.accept_confirm(),
            KeyCode::Enter => {
                if self.confirm_accept {
                    self.accept_confirm();
                } else {
                    self.confirm = None;
                }
            }
            _ => {}
        }
    }

    fn accept_confirm(&mut self) {
        let confirm = self.confirm.take();
        if self.inflight == 0
            && let Some(game) = self.game.as_mut()
        {
            let app_id = game.app_id;
            game.loading = true;
            match confirm {
                Some(Confirm::UnlockAll) => self.send(Job::UnlockAll(app_id)),
                Some(Confirm::Reset) => self.send(Job::Reset(app_id)),
                None => {}
            }
        }
    }

    fn on_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => self.help = false,
            KeyCode::Char('q') => self.quit = true,
            _ => {}
        }
    }

    fn open_settings(&mut self) {
        self.settings_index = 0;
        self.appearance = false;
        self.settings = true;
    }

    fn on_settings_key(&mut self, key: KeyEvent) {
        if self.appearance {
            self.on_appearance_key(key);
            return;
        }
        let last = Language::ALL.len() + 1;
        match key.code {
            KeyCode::Esc | KeyCode::Char('s') => self.settings = false,
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.settings_index = self.settings_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.settings_index < last {
                    self.settings_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.apply_settings_row(),
            _ => {}
        }
    }

    fn apply_settings_row(&mut self) {
        if self.settings_index == 0 {
            self.open_appearance();
            return;
        }
        if self.settings_index == Language::ALL.len() + 1 {
            self.check_for_update();
            return;
        }
        let index = self.settings_index - 1;
        if let Some(language) = Language::ALL.get(index) {
            self.language = *language;
            self.store();
        }
    }

    fn restore(&mut self) {
        let Some(path) = settings_path() else {
            return;
        };
        let Ok(raw) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(saved) = serde_json::from_str::<TuiSettings>(&raw) else {
            return;
        };
        if let Some(theme) = Theme::from_name(&saved.theme) {
            self.theme = theme;
            self.theme_saved = theme;
            self.theme_index = Theme::ALL
                .iter()
                .position(|item| *item == theme)
                .unwrap_or(0);
        }
        if let Some(language) = Language::from_code(&saved.language) {
            self.language = language;
        }
    }

    fn store(&self) {
        let Some(path) = settings_path() else {
            return;
        };
        let theme = if self.appearance {
            self.theme_saved
        } else {
            self.theme
        };
        let saved = TuiSettings {
            theme: theme.name().to_owned(),
            language: self.language.code().to_owned(),
        };
        if let Ok(raw) = serde_json::to_string(&saved) {
            let _ = std::fs::write(path, raw);
        }
    }

    fn check_for_update(&mut self) {
        if self.update_check == UpdateCheck::Checking {
            return;
        }
        self.update_check = UpdateCheck::Checking;
        let tx = self.update_tx.clone();
        thread::spawn(move || {
            let _ = tx.send(query_update());
        });
    }

    fn open_appearance(&mut self) {
        self.theme_saved = self.theme;
        self.theme_query.clear();
        self.theme_index = Theme::ALL
            .iter()
            .position(|theme| *theme == self.theme)
            .unwrap_or(0);
        self.appearance = true;
    }

    fn on_appearance_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.theme = self.theme_saved;
                self.appearance = false;
            }
            KeyCode::Enter => {
                self.theme_saved = self.theme;
                self.appearance = false;
                self.store();
            }
            KeyCode::Up => self.move_theme(-1),
            KeyCode::Down => self.move_theme(1),
            KeyCode::PageUp => self.move_theme(-10),
            KeyCode::PageDown => self.move_theme(10),
            KeyCode::Home => self.move_theme(isize::MIN),
            KeyCode::End => self.move_theme(isize::MAX),
            KeyCode::Backspace => {
                self.theme_query.pop();
                self.sync_theme_filter();
            }
            KeyCode::Char(ch) => {
                self.theme_query.push(ch);
                self.sync_theme_filter();
            }
            _ => {}
        }
    }

    fn filtered_themes(&self) -> Vec<Theme> {
        let query = self.theme_query.to_lowercase();
        Theme::ALL
            .into_iter()
            .filter(|theme| query.is_empty() || theme.name().contains(&query))
            .collect()
    }

    fn move_theme(&mut self, delta: isize) {
        let themes = self.filtered_themes();
        if themes.is_empty() {
            return;
        }
        let len = themes.len() as isize;
        let next = if delta == isize::MIN {
            0
        } else if delta == isize::MAX {
            len - 1
        } else {
            (self.theme_index as isize + delta).rem_euclid(len)
        } as usize;
        self.theme_index = next;
        self.theme = themes[next];
    }

    fn sync_theme_filter(&mut self) {
        let themes = self.filtered_themes();
        if self.theme_query.is_empty() {
            self.theme = self.theme_saved;
            self.theme_index = themes
                .iter()
                .position(|theme| *theme == self.theme_saved)
                .unwrap_or(0);
            return;
        }
        if let Some(theme) = themes.first().copied() {
            self.theme_index = 0;
            self.theme = theme;
        }
    }

    fn on_typing_key(&mut self, typing: Typing, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                if typing == Typing::StatValue && key.code == KeyCode::Enter {
                    self.save_selected_stat();
                }
                self.typing = None;
            }
            KeyCode::Backspace => {
                self.buffer_mut(typing).pop();
            }
            KeyCode::Char(ch) => self.buffer_mut(typing).push(ch),
            _ => {}
        }
    }

    fn buffer_mut(&mut self, typing: Typing) -> &mut String {
        match typing {
            Typing::Search => &mut self.search,
            Typing::AchievementFilter => {
                &mut self
                    .game
                    .as_mut()
                    .expect("achievement filter without a game")
                    .achievement_query
            }
            Typing::StatFilter => {
                &mut self
                    .game
                    .as_mut()
                    .expect("stat filter without a game")
                    .stat_query
            }
            Typing::StatValue => {
                let game = self.game.as_mut().expect("stat edit without a game");
                let id = visible_stats(game)
                    .get(game.stat_index)
                    .map(|index| game.stats[*index].id().to_string())
                    .unwrap_or_default();
                game.drafts.entry(id).or_default()
            }
        }
    }

    fn refresh(&mut self) {
        if self.pane == Pane::Game
            && let Some(game) = self.game.as_mut()
        {
            let app_id = game.app_id;
            game.loading = true;
            self.send(Job::Reload(app_id));
        } else {
            self.send(Job::LoadApps);
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.pane == Pane::Library {
            let len = self.visible_apps().len() as isize;
            if len == 0 {
                return;
            }
            let next = (self.library_index as isize)
                .saturating_add(delta)
                .clamp(0, len - 1);
            self.library_index = next as usize;
            return;
        }
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let len = match game.tab {
            GameTab::Achievements => visible_achievements(game).len(),
            GameTab::Stats => visible_stats(game).len(),
        } as isize;
        if len == 0 {
            return;
        }
        let index = match game.tab {
            GameTab::Achievements => &mut game.achievement_index,
            GameTab::Stats => &mut game.stat_index,
        };
        *index = (*index as isize).saturating_add(delta).clamp(0, len - 1) as usize;
    }

    fn activate(&mut self) {
        if self.inflight > 0 {
            return;
        }
        if self.pane == Pane::Library {
            let Some(app) = self.visible_apps().get(self.library_index).cloned() else {
                return;
            };
            self.open_app(app);
            return;
        }
        if self
            .game
            .as_ref()
            .is_some_and(|game| game.tab == GameTab::Stats)
        {
            self.typing = Some(Typing::StatValue);
        } else {
            self.toggle_selected_achievement();
        }
    }

    fn open_app(&mut self, app: AppModel) {
        if self
            .game
            .as_ref()
            .is_some_and(|game| game.app_id == app.app_id)
        {
            self.pane = Pane::Game;
            return;
        }
        self.game = Some(GameView {
            app_id: app.app_id,
            name: app.app_name,
            developer: app.developer,
            image_url: app.image_url,
            kind: app.app_type,
            tab: GameTab::Achievements,
            loading: true,
            achievements: Vec::new(),
            stats: Vec::new(),
            drafts: HashMap::new(),
            achievement_query: String::new(),
            stat_query: String::new(),
            achievement_index: 0,
            stat_index: 0,
        });
        self.pane = Pane::Game;
        self.send(Job::Open(app.app_id));
    }

    fn close_game(&mut self) {
        self.game = None;
        self.pane = Pane::Library;
        self.typing = None;
    }

    fn switch_tab(&mut self) {
        if let Some(game) = self.game.as_mut() {
            game.tab = match game.tab {
                GameTab::Achievements => GameTab::Stats,
                GameTab::Stats => GameTab::Achievements,
            };
        }
    }

    fn toggle_selected_achievement(&mut self) {
        if self.inflight > 0 {
            return;
        }
        let Some(game) = self.game.as_mut() else {
            return;
        };
        if game.tab != GameTab::Achievements || game.loading {
            return;
        }
        let Some(index) = visible_achievements(game)
            .get(game.achievement_index)
            .copied()
        else {
            return;
        };
        let achievement = &game.achievements[index];
        if achievement.permission != 0 {
            return;
        }
        let id = achievement.id.clone();
        let unlocked = !achievement.is_achieved;
        let app_id = game.app_id;
        game.loading = true;
        self.send(Job::SetAchievement {
            app_id,
            id,
            unlocked,
        });
    }

    fn save_selected_stat(&mut self) {
        if self.inflight > 0 {
            return;
        }
        let Some(game) = self.game.as_ref() else {
            return;
        };
        let Some(index) = visible_stats(game).get(game.stat_index).copied() else {
            return;
        };
        let stat = &game.stats[index];
        if stat.permission() & 2 != 0 {
            return;
        }
        let id = stat.id().to_string();
        let draft = game.drafts.get(&id).cloned().unwrap_or_default();
        let value = match stat {
            StatInfo::Integer(_) => match draft.trim().parse::<i32>() {
                Ok(value) => StatValue::Int(value),
                Err(_) => {
                    self.notice = Some(Err(Message::NotInteger(draft.trim().to_string())));
                    return;
                }
            },
            StatInfo::Float(_) => match draft.trim().parse::<f32>() {
                Ok(value) => StatValue::Float(value),
                Err(_) => {
                    self.notice = Some(Err(Message::NotNumber(draft.trim().to_string())));
                    return;
                }
            },
        };
        let app_id = game.app_id;
        if let Some(game) = self.game.as_mut() {
            game.loading = true;
        }
        self.send(Job::SaveStat { app_id, id, value });
    }

    fn launch(&mut self) {
        let Some(app_id) = self.game.as_ref().map(|game| game.app_id) else {
            return;
        };
        let url = format!("steam://run/{app_id}");
        let spawned = Command::new("open").arg(&url).spawn();
        self.notice = Some(match spawned {
            Ok(_) => Ok(Message::Phrase(Id::OpeningGame)),
            Err(err) => Err(Message::OpenFailed(err.to_string())),
        });
    }

    fn visible_apps(&self) -> Vec<AppModel> {
        let query = self.search.to_lowercase();
        self.apps
            .iter()
            .filter(|app| {
                if self.hide_junk && matches!(app.app_type, AppModelType::Junk) {
                    return false;
                }
                query.is_empty()
                    || app.app_name.to_lowercase().contains(&query)
                    || app.app_id.to_string().contains(&query)
                    || app.developer.to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    fn overlay(&self) -> bool {
        self.settings || self.help || self.confirm.is_some()
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.images.begin_frame();
        let area = frame.area();
        let background = palette(self.theme).background;
        frame.render_widget(
            Block::default().style(Style::default().bg(background)),
            area,
        );
        let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(area);
        let library_width = (area.width as u32 * 34 / 100).clamp(28, 42) as u16;
        let columns = Layout::horizontal([
            Constraint::Length(library_width.min(area.width.saturating_sub(24))),
            Constraint::Min(24),
        ])
        .split(rows[0]);
        self.draw_library(frame, columns[0]);
        self.draw_game(frame, columns[1]);
        self.draw_status(frame, rows[1]);
        if self.help {
            self.draw_help(frame, area);
        }
        if self.settings {
            if self.appearance {
                self.draw_appearance(frame, area);
            } else {
                self.draw_settings(frame, area);
            }
        }
        if self.confirm.is_some() {
            self.draw_confirm(frame, area);
        }
    }

    fn draw_library(&mut self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let focused = self.pane == Pane::Library && !self.overlay();
        let block = library_block(text::t(self.language, Id::Library), focused, &palette);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let preview_height = if inner.height >= MIN_HEIGHT_FOR_PREVIEW {
            CAPSULE_ROWS + 1
        } else {
            0
        };
        let parts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(preview_height),
            Constraint::Length(1),
        ])
        .split(inner);

        frame.render_widget(
            Paragraph::new(query_line(
                text::t(self.language, Id::SearchGames),
                &self.search,
                self.typing == Some(Typing::Search),
                &palette,
            )),
            parts[0],
        );
        let apps = self.visible_apps();
        let mark = if self.hide_junk { "[x]" } else { "[ ]" };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("{mark} {}", text::t(self.language, Id::HideJunk)),
                    palette.dim,
                ),
                Span::styled(
                    format!("   {}", text::shown(self.language, apps.len())),
                    palette.dim,
                ),
            ])),
            parts[1],
        );

        if self.library_index >= apps.len() {
            self.library_index = apps.len().saturating_sub(1);
        }
        let open_id = self.game.as_ref().map(|game| game.app_id);
        if apps.is_empty() {
            let message = if self.inflight > 0 {
                text::t(self.language, Id::LoadingLibrary)
            } else {
                ""
            };
            frame.render_widget(
                Paragraph::new(message)
                    .style(palette.dim)
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: true }),
                parts[2],
            );
        } else {
            let items: Vec<ListItem> = apps
                .iter()
                .map(|app| {
                    let open = open_id == Some(app.app_id);
                    let marker = if open { "● " } else { "  " };
                    let mut label = format!("{marker}{}", app.app_name);
                    if !matches!(app.app_type, AppModelType::App) {
                        label.push_str(" · ");
                        label.push_str(text::t(self.language, kind_id(app.app_type)));
                    }
                    ListItem::new(label)
                })
                .collect();
            let mut state = ListState::default();
            state.select(Some(self.library_index));
            frame.render_stateful_widget(
                List::new(items)
                    .style(palette.fg)
                    .highlight_style(palette.selected)
                    .highlight_symbol(""),
                parts[2],
                &mut state,
            );
        }
        if preview_height > 0 {
            for app in neighbours(&apps, self.library_index) {
                if let Some(url) = app.image_url.as_deref() {
                    self.images.prefetch(url);
                }
            }
        }
        if !self.overlay()
            && let Some(url) = apps
                .get(self.library_index)
                .and_then(|app| app.image_url.as_deref())
        {
            let preview = Rect {
                y: parts[3].y + 1,
                height: parts[3].height.saturating_sub(1),
                ..parts[3]
            };
            self.images
                .render(frame, preview, Slot::LibraryPreview, url);
        }
        frame.render_widget(
            Paragraph::new(format!("Sam {}", env!("CARGO_PKG_VERSION"))).style(palette.dim),
            parts[4],
        );
    }

    fn draw_game(&mut self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let focused = self.pane == Pane::Game && !self.overlay();
        let title = self
            .game
            .as_ref()
            .map(|game| game.name.as_str())
            .unwrap_or("");
        let block = bordered(title, focused, &palette);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let show_images = !self.overlay();
        let Some(game) = self.game.as_mut() else {
            let body =
                Layout::vertical([Constraint::Percentage(35), Constraint::Min(1)]).split(inner);
            frame.render_widget(
                Paragraph::new(text::t(self.language, Id::SelectGame))
                    .style(palette.dim)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                body[1],
            );
            return;
        };
        let language = self.language;
        let header = format!(
            "{} · {} · {}",
            game.app_id,
            text::t(language, kind_id(game.kind)),
            game.developer
        );
        let achievements_label = text::t(language, Id::Achievements);
        let stats_label = text::t(language, Id::Stats);
        let tab = game.tab;
        let capsule = game
            .image_url
            .as_deref()
            .filter(|_| inner.width >= MIN_WIDTH_FOR_HEADER_CAPSULE);
        let header_height = if capsule.is_some() { CAPSULE_ROWS + 1 } else { 1 };
        let detail_height = match tab {
            GameTab::Achievements => ICON_ROWS,
            GameTab::Stats => 3,
        };
        let parts = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(detail_height),
        ])
        .split(inner);
        if let Some(url) = capsule {
            let header_parts = Layout::horizontal([
                Constraint::Length(CAPSULE_COLS),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(Rect {
                height: CAPSULE_ROWS,
                ..parts[0]
            });
            if show_images {
                self.images
                    .render(frame, header_parts[0], Slot::GameHeader, url);
            }
            frame.render_widget(
                Paragraph::new(header)
                    .style(palette.dim)
                    .wrap(Wrap { trim: true }),
                header_parts[2],
            );
        } else {
            frame.render_widget(Paragraph::new(header).style(palette.dim), parts[0]);
        }
        let tabs = Line::from(vec![
            Span::styled(
                format!(" {achievements_label} "),
                if tab == GameTab::Achievements {
                    palette.selected
                } else {
                    palette.dim
                },
            ),
            Span::styled("  ", palette.fg),
            Span::styled(
                format!(" {stats_label} "),
                if tab == GameTab::Stats {
                    palette.selected
                } else {
                    palette.dim
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(tabs), parts[1]);

        match tab {
            GameTab::Achievements => {
                let unlocked = game
                    .achievements
                    .iter()
                    .filter(|achievement| achievement.is_achieved)
                    .count();
                let bar = progress_bar(unlocked, game.achievements.len(), 10);
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(
                            format!(
                                "{}  {bar}  ",
                                text::unlocked(language, unlocked, game.achievements.len())
                            ),
                            palette.dim,
                        ),
                        query_span(
                            text::t(language, Id::FilterAchievements),
                            &game.achievement_query,
                            self.typing == Some(Typing::AchievementFilter),
                            &palette,
                        ),
                    ])),
                    parts[2],
                );
                let visible = visible_achievements(game);
                if game.achievement_index >= visible.len() {
                    game.achievement_index = visible.len().saturating_sub(1);
                }
                let items: Vec<ListItem> = visible
                    .iter()
                    .map(|index| {
                        let achievement = &game.achievements[*index];
                        let mark = if achievement.permission != 0 {
                            "[-]"
                        } else if achievement.is_achieved {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        let name = if achievement.name.is_empty() {
                            achievement.id.as_str()
                        } else {
                            achievement.name.as_str()
                        };
                        let percent = achievement
                            .global_achieved_percent
                            .map(|percent| format!("  {percent:.0}%"))
                            .unwrap_or_default();
                        ListItem::new(format!("{mark} {name}{percent}"))
                    })
                    .collect();
                let mut state = ListState::default();
                if !visible.is_empty() {
                    state.select(Some(game.achievement_index));
                }
                frame.render_stateful_widget(
                    List::new(items)
                        .style(palette.fg)
                        .highlight_style(palette.selected)
                        .highlight_symbol(""),
                    parts[3],
                    &mut state,
                );
                let selected = visible
                    .get(game.achievement_index)
                    .map(|index| &game.achievements[*index]);
                let detail_parts = Layout::horizontal([
                    Constraint::Length(ICON_COLS),
                    Constraint::Length(2),
                    Constraint::Min(1),
                ])
                .split(parts[4]);
                for index in neighbours(&visible, game.achievement_index) {
                    if let Some(url) = achievement_icon_url(&game.achievements[*index]) {
                        self.images.prefetch(url);
                    }
                }
                if show_images
                    && let Some(url) = selected.and_then(achievement_icon_url)
                {
                    self.images
                        .render(frame, detail_parts[0], Slot::AchievementIcon, url);
                }
                let detail = selected
                    .map(|achievement| achievement_detail(achievement, language))
                    .unwrap_or_default();
                frame.render_widget(
                    Paragraph::new(detail)
                        .style(palette.dim)
                        .wrap(Wrap { trim: true }),
                    detail_parts[2],
                );
            }
            GameTab::Stats => {
                frame.render_widget(
                    Paragraph::new(query_line(
                        text::t(language, Id::FilterStats),
                        &game.stat_query,
                        self.typing == Some(Typing::StatFilter),
                        &palette,
                    )),
                    parts[2],
                );
                let visible = visible_stats(game);
                if game.stat_index >= visible.len() {
                    game.stat_index = visible.len().saturating_sub(1);
                }
                let editing = self.typing == Some(Typing::StatValue);
                let items: Vec<ListItem> = visible
                    .iter()
                    .enumerate()
                    .map(|(row, index)| {
                        let stat = &game.stats[*index];
                        let id = stat.id();
                        let name = if stat.display_name().is_empty() {
                            id
                        } else {
                            stat.display_name()
                        };
                        let draft = game.drafts.get(id).map(String::as_str).unwrap_or("");
                        let protected = stat.permission() & 2 != 0;
                        let cursor = if editing && row == game.stat_index {
                            "▍"
                        } else {
                            ""
                        };
                        let value = if protected {
                            draft.to_string()
                        } else {
                            format!("[{draft}{cursor}]")
                        };
                        ListItem::new(format!("{name}  {value}"))
                    })
                    .collect();
                let mut state = ListState::default();
                if !visible.is_empty() {
                    state.select(Some(game.stat_index));
                }
                frame.render_stateful_widget(
                    List::new(items)
                        .style(palette.fg)
                        .highlight_style(palette.selected)
                        .highlight_symbol(""),
                    parts[3],
                    &mut state,
                );
                let detail = visible
                    .get(game.stat_index)
                    .map(|index| stat_detail(&game.stats[*index], language))
                    .unwrap_or_default();
                frame.render_widget(
                    Paragraph::new(detail)
                        .style(palette.dim)
                        .wrap(Wrap { trim: true }),
                    parts[4],
                );
            }
        }
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(palette.border)
            .style(palette.fg);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);
        let message = if self.inflight > 0 && !matches!(self.notice, Some(Err(_))) {
            text::t(self.language, Id::TalkingToSteam).to_string()
        } else {
            match &self.notice {
                Some(Ok(message)) => message.text(self.language),
                Some(Err(message)) => message.text(self.language),
                None => text::t(self.language, Id::Ready).to_string(),
            }
        };
        let style = if matches!(self.notice, Some(Err(_))) && self.inflight == 0 {
            palette.error
        } else {
            palette.fg
        };
        frame.render_widget(Paragraph::new(message).style(style), rows[0]);
        frame.render_widget(Paragraph::new(self.hint_line(&palette)), rows[1]);
    }

    fn hint_line(&self, palette: &Palette) -> Line<'static> {
        let language = self.language;
        let commands: Vec<(&str, String)> = if self.confirm.is_some() {
            vec![
                ("←→", String::new()),
                ("enter", String::new()),
                ("esc", text::t(language, Id::Cancel).to_string()),
            ]
        } else if self.appearance {
            vec![
                ("↑↓", String::new()),
                ("enter", String::new()),
                ("esc", text::t(language, Id::Cancel).to_string()),
            ]
        } else if self.settings || self.help {
            vec![
                ("↑↓", String::new()),
                ("enter", String::new()),
                ("esc", text::t(language, Id::Cancel).to_string()),
            ]
        } else if self.typing == Some(Typing::StatValue) {
            vec![
                ("enter", text::t(language, Id::Save).to_string()),
                ("esc", text::t(language, Id::Cancel).to_string()),
            ]
        } else if self.typing.is_some() {
            vec![("esc", text::t(language, Id::Cancel).to_string())]
        } else if self.pane == Pane::Library {
            vec![
                ("?", text::t(language, Id::Help).to_string()),
                ("/", text::t(language, Id::SearchGames).to_string()),
                ("r", text::t(language, Id::Refresh).to_string()),
                ("h", text::t(language, Id::HideJunk).to_string()),
                ("s", text::t(language, Id::Settings).to_string()),
                ("q", text::t(language, Id::Quit).to_string()),
            ]
        } else if self
            .game
            .as_ref()
            .is_some_and(|game| game.tab == GameTab::Stats)
        {
            vec![
                ("esc", text::t(language, Id::BackLibrary).to_string()),
                ("tab", text::t(language, Id::Achievements).to_string()),
                ("enter", text::t(language, Id::Save).to_string()),
                ("/", text::t(language, Id::FilterStats).to_string()),
                ("r", text::t(language, Id::Refresh).to_string()),
                ("?", text::t(language, Id::Help).to_string()),
            ]
        } else {
            vec![
                ("esc", text::t(language, Id::BackLibrary).to_string()),
                ("tab", text::t(language, Id::Stats).to_string()),
                ("space", String::new()),
                ("u", text::t(language, Id::UnlockAll).to_string()),
                ("x", text::t(language, Id::ResetAll).to_string()),
                ("l", text::t(language, Id::Launch).to_string()),
                ("?", text::t(language, Id::Help).to_string()),
            ]
        };
        let mut spans = Vec::new();
        for (index, (key, label)) in commands.iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled("  ", palette.fg));
            }
            spans.push(Span::styled(
                (*key).to_string(),
                palette.focus.add_modifier(Modifier::BOLD),
            ));
            if !label.is_empty() {
                spans.push(Span::styled(format!(" {label}"), palette.dim));
            }
        }
        Line::from(spans)
    }

    fn draw_settings(&self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let popup = centered(area, 62, 18);
        frame.render_widget(Clear, popup);
        let block = bordered(text::t(self.language, Id::Settings), true, &palette);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let mut lines = vec![
            menu_row(
                self.settings_index == 0,
                text::t(self.language, Id::Appearance),
                self.theme.name(),
                &palette,
            ),
            Line::from(""),
            Line::from(text::t(self.language, Id::Language)).style(palette.dim),
        ];
        for (index, language) in Language::ALL.iter().enumerate() {
            lines.push(settings_row(
                self.settings_index == index + 1,
                self.language == *language,
                language.native_name(),
                &palette,
            ));
        }
        lines.push(Line::from(""));
        lines.push(menu_row(
            self.settings_index == Language::ALL.len() + 1,
            text::t(self.language, Id::CheckUpdates),
            self.update_label(),
            &palette,
        ));

        let languages = Language::ALL.len();
        let cursor = if self.settings_index == 0 {
            0
        } else if self.settings_index <= languages {
            self.settings_index + 2
        } else {
            self.settings_index + 3
        };
        let height = inner.height as usize;
        let offset = cursor.saturating_sub(height.saturating_sub(1));
        let visible: Vec<Line> = lines.into_iter().skip(offset).collect();
        frame.render_widget(Paragraph::new(visible), inner);
    }

    fn update_label(&self) -> &'static str {
        let id = match self.update_check {
            UpdateCheck::Idle => return env!("CARGO_PKG_VERSION"),
            UpdateCheck::Checking => Id::CheckingUpdates,
            UpdateCheck::Current => Id::UpToDate,
            UpdateCheck::Available => Id::UpdateAvailable,
            UpdateCheck::Failed => Id::UpdateCheckFailed,
        };
        text::t(self.language, id)
    }

    fn draw_appearance(&self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let popup = centered(area, 42, 20);
        frame.render_widget(Clear, popup);
        let block = bordered(text::t(self.language, Id::Appearance), true, &palette);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
        let search = if self.theme_query.is_empty() {
            Line::from(vec![
                Span::styled("/ ", palette.dim),
                Span::styled(text::t(self.language, Id::Search), palette.dim),
                Span::styled("▍", palette.focus),
            ])
        } else {
            Line::from(Span::styled(format!("/ {}▍", self.theme_query), palette.fg))
        };
        frame.render_widget(Paragraph::new(search), parts[0]);

        let themes = self.filtered_themes();
        let height = parts[1].height as usize;
        let offset = self.theme_index.saturating_sub(height.saturating_sub(1));
        let lines: Vec<Line> = themes
            .iter()
            .enumerate()
            .skip(offset)
            .take(height)
            .map(|(index, theme)| {
                let active = index == self.theme_index;
                let current = *theme == self.theme_saved;
                let style = if active {
                    palette.focus.add_modifier(Modifier::BOLD)
                } else if current {
                    palette.focus
                } else {
                    palette.fg
                };
                Line::from(format!(" ● {}", theme.name())).style(style)
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), parts[1]);
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let popup = centered(area, 62, 20);
        frame.render_widget(Clear, popup);
        let block = bordered(text::t(self.language, Id::Help), true, &palette);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let language = self.language;
        let lines = vec![
            Line::from(text::t(language, Id::Library)).style(palette.focus),
            help_row("?", text::t(language, Id::Help), &palette),
            help_row("/", text::t(language, Id::SearchGames), &palette),
            help_row("↑↓", "j k", &palette),
            help_row("r", text::t(language, Id::Refresh), &palette),
            help_row("h", text::t(language, Id::HideJunk), &palette),
            help_row("s", text::t(language, Id::Settings), &palette),
            help_row("q", text::t(language, Id::Quit), &palette),
            Line::from(""),
            Line::from(text::t(language, Id::Achievements)).style(palette.focus),
            help_row("esc", text::t(language, Id::BackLibrary), &palette),
            help_row("tab", text::t(language, Id::Stats), &palette),
            help_row("space", "[x]", &palette),
            help_row("u", text::t(language, Id::UnlockAll), &palette),
            help_row("x", text::t(language, Id::ResetAll), &palette),
            help_row("l", text::t(language, Id::Launch), &palette),
            help_row("enter", text::t(language, Id::Save), &palette),
        ];
        let height = inner.height as usize;
        frame.render_widget(
            Paragraph::new(lines.into_iter().take(height).collect::<Vec<_>>()),
            inner,
        );
    }

    fn draw_confirm(&self, frame: &mut Frame, area: Rect) {
        let Some(confirm) = self.confirm else {
            return;
        };
        let palette = palette(self.theme);
        let popup = centered(area, 62, 8);
        frame.render_widget(Clear, popup);
        let block = bordered(text::t(self.language, Id::Confirm), true, &palette);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let question = match confirm {
            Confirm::UnlockAll => text::t(self.language, Id::UnlockPrompt),
            Confirm::Reset => text::t(self.language, Id::ResetPrompt),
        };
        let parts = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
        frame.render_widget(
            Paragraph::new(question)
                .style(palette.fg)
                .wrap(Wrap { trim: true }),
            parts[0],
        );
        let cancel = button(
            text::t(self.language, Id::Cancel),
            !self.confirm_accept,
            &palette,
        );
        let continue_label = button(
            text::t(self.language, Id::Continue),
            self.confirm_accept,
            &palette,
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![cancel, Span::raw("   "), continue_label])),
            parts[1],
        );
    }
}

struct Palette {
    fg: Style,
    dim: Style,
    error: Style,
    selected: Style,
    border: Style,
    focus: Style,
    background: Color,
}

fn palette(theme: Theme) -> Palette {
    let (text, muted, error, primary, background, border) = match theme {
        Theme::Aura => c(
            237, 236, 238, 109, 109, 109, 255, 103, 103, 162, 119, 255, 15, 15, 15, 45, 45, 45,
        ),
        Theme::Ayu => c(
            191, 189, 182, 86, 91, 102, 217, 87, 87, 89, 194, 255, 11, 14, 20, 108, 115, 128,
        ),
        Theme::Carbonfox => c(
            242, 244, 248, 125, 132, 143, 238, 83, 150, 51, 177, 255, 22, 22, 22, 48, 48, 48,
        ),
        Theme::Catppuccin => c(
            205, 214, 244, 147, 153, 178, 243, 139, 168, 137, 180, 250, 30, 30, 46, 49, 50, 68,
        ),
        Theme::CatppuccinFrappe => c(
            198, 208, 245, 148, 156, 184, 231, 130, 132, 141, 164, 226, 48, 52, 70, 65, 69, 89,
        ),
        Theme::CatppuccinMacchiato => c(
            202, 211, 245, 147, 154, 183, 237, 135, 150, 138, 173, 244, 36, 39, 58, 54, 58, 79,
        ),
        Theme::Cobalt2 => c(
            255, 255, 255, 173, 183, 201, 255, 0, 136, 0, 136, 255, 25, 53, 73, 31, 70, 98,
        ),
        Theme::Cursor => c(
            228, 228, 228, 228, 228, 228, 227, 70, 113, 136, 192, 208, 24, 24, 24, 228, 228, 228,
        ),
        Theme::Dracula => c(
            248, 248, 242, 98, 114, 164, 255, 85, 85, 189, 147, 249, 40, 42, 54, 68, 71, 90,
        ),
        Theme::Everforest => c(
            211, 198, 170, 122, 132, 120, 230, 126, 128, 167, 192, 128, 45, 53, 59, 133, 146, 137,
        ),
        Theme::Flexoki => c(
            206, 205, 195, 111, 110, 105, 209, 77, 65, 218, 112, 44, 16, 15, 15, 87, 86, 83,
        ),
        Theme::Github => c(
            201, 209, 217, 139, 148, 158, 248, 81, 73, 88, 166, 255, 13, 17, 23, 48, 54, 61,
        ),
        Theme::Gruvbox => c(
            235, 219, 178, 146, 131, 116, 251, 73, 52, 131, 165, 152, 40, 40, 40, 102, 92, 84,
        ),
        Theme::Kanagawa => c(
            220, 215, 186, 114, 113, 105, 232, 36, 36, 126, 156, 216, 31, 31, 40, 84, 84, 109,
        ),
        Theme::LucentOrng => c(
            238, 238, 238, 128, 128, 128, 224, 108, 117, 236, 91, 43, 42, 26, 21, 236, 91, 43,
        ),
        Theme::Material => c(
            238, 255, 255, 84, 110, 122, 240, 113, 120, 130, 170, 255, 38, 50, 56, 55, 71, 79,
        ),
        Theme::Matrix => c(
            98, 255, 148, 140, 163, 145, 255, 75, 75, 46, 255, 106, 10, 14, 10, 30, 42, 27,
        ),
        Theme::Mercury => c(
            221, 221, 229, 157, 157, 168, 252, 146, 180, 141, 164, 245, 23, 23, 33, 180, 183, 200,
        ),
        Theme::Monokai => c(
            248, 248, 242, 117, 113, 94, 249, 38, 114, 102, 217, 239, 39, 40, 34, 62, 61, 50,
        ),
        Theme::Nightowl => c(
            214, 222, 235, 95, 126, 151, 239, 83, 80, 130, 170, 255, 1, 22, 39, 95, 126, 151,
        ),
        Theme::Nord => c(
            236, 239, 244, 139, 149, 167, 191, 97, 106, 136, 192, 208, 46, 52, 64, 67, 76, 94,
        ),
        Theme::OneDark => c(
            171, 178, 191, 92, 99, 112, 224, 108, 117, 97, 175, 239, 40, 44, 52, 57, 63, 74,
        ),
        Theme::Opencode => c(
            238, 238, 238, 128, 128, 128, 224, 108, 117, 250, 178, 131, 10, 10, 10, 72, 72, 72,
        ),
        Theme::Orng => c(
            238, 238, 238, 128, 128, 128, 224, 108, 117, 236, 91, 43, 10, 10, 10, 236, 91, 43,
        ),
        Theme::OsakaJade => c(
            193, 196, 151, 83, 104, 91, 255, 83, 69, 45, 213, 183, 17, 28, 24, 61, 74, 68,
        ),
        Theme::Palenight => c(
            166, 172, 205, 103, 110, 149, 240, 113, 120, 130, 170, 255, 41, 45, 62, 50, 54, 74,
        ),
        Theme::Rosepine => c(
            224, 222, 244, 110, 106, 134, 235, 111, 146, 156, 207, 216, 25, 23, 36, 64, 61, 82,
        ),
        Theme::Solarized => c(
            131, 148, 150, 88, 110, 117, 220, 50, 47, 38, 139, 210, 0, 43, 54, 7, 54, 66,
        ),
        Theme::Synthwave84 => c(
            255, 255, 255, 132, 139, 189, 254, 68, 80, 54, 249, 246, 38, 35, 53, 73, 84, 149,
        ),
        Theme::Tokyonight => c(
            200, 211, 245, 130, 139, 184, 255, 117, 127, 130, 170, 255, 26, 27, 38, 115, 122, 162,
        ),
        Theme::Vercel => c(
            237, 237, 237, 135, 135, 135, 229, 72, 77, 0, 112, 243, 0, 0, 0, 31, 31, 31,
        ),
        Theme::Vesper => c(
            255, 255, 255, 160, 160, 160, 255, 128, 128, 255, 199, 153, 16, 16, 16, 40, 40, 40,
        ),
        Theme::Zenburn => c(
            220, 220, 204, 159, 159, 159, 204, 147, 147, 140, 208, 211, 63, 63, 63, 95, 95, 95,
        ),
    };
    let background = Color::Rgb(background.0, background.1, background.2);
    let primary = Color::Rgb(primary.0, primary.1, primary.2);
    Palette {
        fg: Style::default()
            .fg(Color::Rgb(text.0, text.1, text.2))
            .bg(background),
        dim: Style::default()
            .fg(Color::Rgb(muted.0, muted.1, muted.2))
            .bg(background),
        error: Style::default()
            .fg(Color::Rgb(error.0, error.1, error.2))
            .bg(background),
        selected: Style::default()
            .fg(background)
            .bg(primary)
            .add_modifier(Modifier::BOLD),
        border: Style::default()
            .fg(Color::Rgb(border.0, border.1, border.2))
            .bg(background),
        focus: Style::default().fg(primary).bg(background),
        background,
    }
}

fn c(
    tr: u8,
    tg: u8,
    tb: u8,
    mr: u8,
    mg: u8,
    mb: u8,
    er: u8,
    eg: u8,
    eb: u8,
    pr: u8,
    pg: u8,
    pb: u8,
    br: u8,
    bg: u8,
    bb: u8,
    xr: u8,
    xg: u8,
    xb: u8,
) -> (
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
) {
    (
        (tr, tg, tb),
        (mr, mg, mb),
        (er, eg, eb),
        (pr, pg, pb),
        (br, bg, bb),
        (xr, xg, xb),
    )
}

fn query_update() -> UpdateCheck {
    let local = env!("SAM_GIT_COMMIT");
    if local.is_empty() {
        return UpdateCheck::Failed;
    }
    let output = Command::new("git")
        .args([
            "ls-remote",
            "git@github.com:luisitoluis/sam-macos-rs.git",
            "refs/heads/master",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -o BatchMode=yes -o ConnectTimeout=8",
        )
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            let remote = text.split_whitespace().next().unwrap_or("");
            if remote.is_empty() {
                UpdateCheck::Failed
            } else if remote == local {
                UpdateCheck::Current
            } else {
                UpdateCheck::Available
            }
        }
        _ => UpdateCheck::Failed,
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TuiSettings {
    theme: String,
    language: String,
}

fn settings_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let dir = std::path::PathBuf::from(home).join("Library/Application Support/Sam");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("tui-settings.json"))
}

fn library_block(title: &str, focused: bool, palette: &Palette) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(Line::from(format!(" {title} ")).centered())
        .border_style(if focused {
            palette.focus
        } else {
            palette.border
        })
        .style(palette.fg)
}

fn settings_row(cursor: bool, selected: bool, label: &str, palette: &Palette) -> Line<'static> {
    let mark = if selected { "[x]" } else { "[ ]" };
    let pointer = if cursor { ">" } else { " " };
    let style = if cursor { palette.selected } else { palette.fg };
    Line::from(format!("{pointer} {mark} {label}")).style(style)
}

fn menu_row(cursor: bool, label: &str, value: &str, palette: &Palette) -> Line<'static> {
    let pointer = if cursor { ">" } else { " " };
    let style = if cursor { palette.selected } else { palette.fg };
    Line::from(format!("{pointer} {label}   {value}")).style(style)
}

fn bordered(title: &str, focused: bool, palette: &Palette) -> Block<'static> {
    let title = title.to_string();
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if focused {
            palette.focus
        } else {
            palette.border
        })
        .style(palette.fg)
}

fn query_line(placeholder: &str, value: &str, editing: bool, palette: &Palette) -> Line<'static> {
    Line::from(query_span(placeholder, value, editing, palette))
}

fn query_span(placeholder: &str, value: &str, editing: bool, palette: &Palette) -> Span<'static> {
    let cursor = if editing { "▍" } else { "" };
    if value.is_empty() && !editing {
        Span::styled(format!("/ {placeholder}"), palette.dim)
    } else {
        Span::styled(format!("/ {value}{cursor}"), palette.fg)
    }
}

fn progress_bar(done: usize, total: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let filled = if total == 0 {
        0
    } else {
        done.saturating_mul(width) / total
    };
    let filled = filled.min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// Terminal cells are roughly twice as tall as they are wide. Steam capsules
/// are close to 8:3 and achievement icons are square.
const CAPSULE_ROWS: u16 = 5;
const CAPSULE_COLS: u16 = 27;
const ICON_ROWS: u16 = 4;
const ICON_COLS: u16 = 8;
const MIN_HEIGHT_FOR_PREVIEW: u16 = 18;
const MIN_WIDTH_FOR_HEADER_CAPSULE: u16 = 52;

const PREFETCH_AROUND: usize = 4;

/// Items near `selected`, farthest first, so the fetcher (newest first) loads
/// the closest ones before the rest.
fn neighbours<T>(items: &[T], selected: usize) -> impl Iterator<Item = &T> {
    (1..=PREFETCH_AROUND).rev().flat_map(move |distance| {
        let before = selected.checked_sub(distance).and_then(|index| items.get(index));
        let after = items.get(selected + distance);
        before.into_iter().chain(after)
    })
}

fn achievement_icon_url(achievement: &AchievementInfo) -> Option<&str> {
    let url = if achievement.is_achieved || achievement.icon_locked.ends_with('/') {
        &achievement.icon_normal
    } else {
        &achievement.icon_locked
    };
    (url.starts_with("http") && !url.ends_with('/')).then_some(url.as_str())
}

fn achievement_detail(achievement: &AchievementInfo, language: Language) -> String {
    let mut meta = achievement.id.clone();
    if achievement.permission != 0 {
        meta.push_str(" · ");
        meta.push_str(text::t(language, Id::Protected));
    }
    if let Some(percent) = achievement.global_achieved_percent {
        meta.push_str(" · ");
        meta.push_str(&text::of_players(language, percent));
    }
    if achievement.description.is_empty() {
        meta
    } else {
        format!("{}\n{meta}", achievement.description)
    }
}

fn stat_detail(stat: &StatInfo, language: Language) -> String {
    let mut meta = stat.id().to_string();
    if stat.permission() & 2 != 0 {
        meta.push_str(" · ");
        meta.push_str(text::t(language, Id::Protected));
    }
    let increment_only = match stat {
        StatInfo::Integer(stat) => stat.is_increment_only,
        StatInfo::Float(stat) => stat.is_increment_only,
    };
    if increment_only {
        meta.push_str(" · ");
        meta.push_str(text::t(language, Id::IncrementOnly));
    }
    meta
}

fn help_row(key: &str, label: &str, palette: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<6}"), palette.focus),
        Span::styled(label.to_string(), palette.dim),
    ])
}

fn button(label: &str, focused: bool, palette: &Palette) -> Span<'static> {
    let text = format!(" {label} ");
    if focused {
        Span::styled(text, palette.selected)
    } else {
        Span::styled(text, palette.dim)
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn kind_id(kind: AppModelType) -> Id {
    match kind {
        AppModelType::App => Id::KindApp,
        AppModelType::Mod => Id::KindMod,
        AppModelType::Demo => Id::KindDemo,
        AppModelType::Junk => Id::KindJunk,
    }
}

fn visible_achievements(game: &GameView) -> Vec<usize> {
    let query = game.achievement_query.to_lowercase();
    game.achievements
        .iter()
        .enumerate()
        .filter(|(_, achievement)| {
            query.is_empty()
                || achievement.name.to_lowercase().contains(&query)
                || achievement.id.to_lowercase().contains(&query)
                || achievement.description.to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect()
}

fn visible_stats(game: &GameView) -> Vec<usize> {
    let query = game.stat_query.to_lowercase();
    game.stats
        .iter()
        .enumerate()
        .filter(|(_, stat)| {
            query.is_empty()
                || stat.id().to_lowercase().contains(&query)
                || stat.display_name().to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect()
}

fn drafts_from(stats: &[StatInfo]) -> HashMap<String, String> {
    stats
        .iter()
        .map(|stat| {
            let value = match stat {
                StatInfo::Integer(stat) => stat.int_value.to_string(),
                StatInfo::Float(stat) => stat.float_value.to_string(),
            };
            (stat.id().to_string(), value)
        })
        .collect()
}
