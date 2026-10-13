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

use super::worker::{self, Job, StatValue, UiEvent};
use crate::backend::app_lister::{AppModel, AppModelType};
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::text::{self, Id, Language, Message};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::HashMap;
use std::io::{self, stdout};
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    System,
    Dark,
    Light,
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
    confirm: Option<Confirm>,
    notice: Option<Result<Message, Message>>,
    language: Language,
    theme: Theme,
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
        app.send(Job::LoadApps);
        let result = event_loop(&mut terminal, &mut app);
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
            settings_index: 1,
            confirm: None,
            notice: Some(Ok(Message::Phrase(Id::LoadingLibrary))),
            language: Language::from_locale(),
            theme: Theme::Dark,
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
        if let Some(typing) = self.typing {
            self.on_typing_key(typing, key);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('s') => self.open_settings(),
            KeyCode::Char('h') if self.pane == Pane::Library => self.hide_junk = !self.hide_junk,
            KeyCode::Char('r') if self.inflight == 0 => self.refresh(),
            KeyCode::Char('/') => self.typing = Some(match self.pane {
                Pane::Library => Typing::Search,
                Pane::Game => match self.game.as_ref().map(|game| game.tab) {
                    Some(GameTab::Stats) => Typing::StatFilter,
                    _ => Typing::AchievementFilter,
                },
            }),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Enter => self.activate(),
            KeyCode::Esc | KeyCode::Backspace if self.pane == Pane::Game => self.close_game(),
            KeyCode::Tab | KeyCode::BackTab if self.pane == Pane::Game => self.switch_tab(),
            KeyCode::Char(' ') if self.pane == Pane::Game => self.toggle_selected_achievement(),
            KeyCode::Char('u') if self.pane == Pane::Game => self.confirm = Some(Confirm::UnlockAll),
            KeyCode::Char('x') if self.pane == Pane::Game => self.confirm = Some(Confirm::Reset),
            KeyCode::Char('l') if self.pane == Pane::Game => self.launch(),
            KeyCode::Left if self.pane == Pane::Game => self.pane = Pane::Library,
            KeyCode::Right if self.game.is_some() => self.pane = Pane::Game,
            _ => {}
        }
    }

    fn on_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => self.confirm = None,
            KeyCode::Enter | KeyCode::Char('y') => {
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
            _ => {}
        }
    }

    fn open_settings(&mut self) {
        self.settings_index = match self.theme {
            Theme::System => 0,
            Theme::Dark => 1,
            Theme::Light => 2,
        };
        self.settings = true;
    }

    fn on_settings_key(&mut self, key: KeyEvent) {
        let last = 3 + Language::ALL.len() - 1;
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
        if self.settings_index < 3 {
            self.theme = [Theme::System, Theme::Dark, Theme::Light][self.settings_index];
            return;
        }
        let index = self.settings_index - 3;
        if let Some(language) = Language::ALL.get(index) {
            self.language = *language;
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
            Typing::AchievementFilter => &mut self
                .game
                .as_mut()
                .expect("achievement filter without a game")
                .achievement_query,
            Typing::StatFilter => &mut self
                .game
                .as_mut()
                .expect("stat filter without a game")
                .stat_query,
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
            let next = (self.library_index as isize + delta).clamp(0, len - 1);
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
        *index = (*index as isize + delta).clamp(0, len - 1) as usize;
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
        let Some(index) = visible_achievements(game).get(game.achievement_index).copied() else {
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
        let spawned = {
            #[cfg(target_os = "macos")]
            {
                Command::new("open").arg(&url).spawn()
            }
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd").args(["/C", "start", "", &url]).spawn()
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                Command::new("xdg-open").arg(&url).spawn()
            }
        };
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

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(area);
        let columns =
            Layout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)]).split(rows[0]);
        self.draw_library(frame, columns[0]);
        self.draw_game(frame, columns[1]);
        self.draw_status(frame, rows[1]);
        if self.settings {
            self.draw_settings(frame, area);
        }
        if self.confirm.is_some() {
            self.draw_confirm(frame, area);
        }
    }

    fn draw_library(&mut self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let focused = self.pane == Pane::Library && !self.settings && self.confirm.is_none();
        let block = library_block(text::t(self.language, Id::Library), focused, &palette);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let parts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        let search = format!(
            "{}: {}{}",
            text::t(self.language, Id::SearchGames),
            self.search,
            if self.typing == Some(Typing::Search) {
                "_"
            } else {
                ""
            }
        );
        frame.render_widget(Paragraph::new(search).style(palette.fg), parts[0]);
        let junk = format!(
            "[{}] {}",
            if self.hide_junk { "x" } else { " " },
            text::t(self.language, Id::HideJunk)
        );
        frame.render_widget(Paragraph::new(junk).style(palette.dim), parts[1]);
        let shown = text::shown(self.language, self.visible_apps().len());
        frame.render_widget(
            Paragraph::new(format!(
                "{}   {}",
                text::t(self.language, Id::Refresh),
                shown
            ))
            .style(palette.dim),
            parts[2],
        );

        let apps = self.visible_apps();
        if self.library_index >= apps.len() {
            self.library_index = apps.len().saturating_sub(1);
        }
        let items: Vec<ListItem> = apps
            .iter()
            .map(|app| {
                let kind = text::t(self.language, kind_id(app.app_type));
                ListItem::new(vec![
                    Line::from(app.app_name.as_str()),
                    Line::from(format!("{} · {kind}", app.app_id)).style(palette.dim),
                ])
            })
            .collect();
        if apps.is_empty() && self.inflight > 0 {
            frame.render_widget(
                Paragraph::new(text::t(self.language, Id::LoadingLibrary)).style(palette.dim),
                parts[3],
            );
        } else {
            let mut state = ListState::default();
            if !apps.is_empty() {
                state.select(Some(self.library_index));
            }
            let list = List::new(items)
                .highlight_style(palette.selected)
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, parts[3], &mut state);
        }
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("Sam {}", env!("CARGO_PKG_VERSION")),
                    palette.dim,
                ),
                Span::styled(
                    format!("   ⚙ {}", text::t(self.language, Id::Settings)),
                    palette.focus,
                ),
            ])),
            parts[4],
        );
    }

    fn draw_game(&mut self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let focused = self.pane == Pane::Game && !self.settings && self.confirm.is_none();
        let title = self
            .game
            .as_ref()
            .map(|game| game.name.as_str())
            .unwrap_or("");
        let block = bordered(title, focused, &palette);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let Some(game) = self.game.as_mut() else {
            frame.render_widget(
                Paragraph::new(text::t(self.language, Id::SelectGame))
                    .style(palette.dim)
                    .wrap(Wrap { trim: true }),
                inner,
            );
            return;
        };
        let language = self.language;
        let header = format!(
            "{} · {} · {}    {}    {}",
            game.app_id,
            text::t(language, kind_id(game.kind)),
            game.developer,
            text::t(language, Id::Launch),
            text::t(language, Id::Refresh)
        );
        let achievements_label = text::t(language, Id::Achievements);
        let stats_label = text::t(language, Id::Stats);
        let tab = game.tab;
        let parts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);
        frame.render_widget(Paragraph::new(header).style(palette.dim), parts[0]);
        let tabs = Line::from(vec![
            Span::styled(
                format!(" {achievements_label} "),
                if tab == GameTab::Achievements {
                    palette.selected
                } else {
                    palette.dim
                },
            ),
            Span::raw(" "),
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
                let summary = format!(
                    "{}   {}   {}   {}: {}{}",
                    text::unlocked(language, unlocked, game.achievements.len()),
                    text::t(language, Id::UnlockAll),
                    text::t(language, Id::ResetAll),
                    text::t(language, Id::FilterAchievements),
                    game.achievement_query,
                    if self.typing == Some(Typing::AchievementFilter) {
                        "_"
                    } else {
                        ""
                    }
                );
                frame.render_widget(Paragraph::new(summary).style(palette.fg), parts[2]);
                let visible = visible_achievements(game);
                if game.achievement_index >= visible.len() {
                    game.achievement_index = visible.len().saturating_sub(1);
                }
                let items: Vec<ListItem> = visible
                    .iter()
                    .map(|index| {
                        let achievement = &game.achievements[*index];
                        let mark = if achievement.is_achieved { "[x]" } else { "[ ]" };
                        let name = if achievement.name.is_empty() {
                            achievement.id.as_str()
                        } else {
                            achievement.name.as_str()
                        };
                        let mut lines = vec![Line::from(format!("{mark} {name}"))];
                        if !achievement.description.is_empty() {
                            lines.push(
                                Line::from(achievement.description.as_str()).style(palette.dim),
                            );
                        }
                        let mut meta = achievement.id.clone();
                        if achievement.permission != 0 {
                            meta.push_str(" · ");
                            meta.push_str(text::t(language, Id::Protected));
                        }
                        if let Some(percent) = achievement.global_achieved_percent {
                            meta.push_str(" · ");
                            meta.push_str(&text::of_players(language, percent));
                        }
                        lines.push(Line::from(meta).style(palette.dim));
                        ListItem::new(lines)
                    })
                    .collect();
                let mut state = ListState::default();
                if !visible.is_empty() {
                    state.select(Some(game.achievement_index));
                }
                let list_area = list_area(frame, parts[3], game.loading, language, &palette);
                frame.render_stateful_widget(
                    List::new(items)
                        .highlight_style(palette.selected)
                        .highlight_symbol("> "),
                    list_area,
                    &mut state,
                );
            }
            GameTab::Stats => {
                let summary = format!(
                    "{}: {}{}",
                    text::t(language, Id::FilterStats),
                    game.stat_query,
                    if self.typing == Some(Typing::StatFilter) {
                        "_"
                    } else {
                        ""
                    }
                );
                frame.render_widget(Paragraph::new(summary).style(palette.fg), parts[2]);
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
                        let cursor = if editing && row == game.stat_index {
                            "_"
                        } else {
                            ""
                        };
                        let mut meta = id.to_string();
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
                        ListItem::new(vec![
                            Line::from(format!("{name}  {draft}{cursor}")),
                            Line::from(meta).style(palette.dim),
                        ])
                    })
                    .collect();
                let mut state = ListState::default();
                if !visible.is_empty() {
                    state.select(Some(game.stat_index));
                }
                let list_area = list_area(frame, parts[3], game.loading, language, &palette);
                frame.render_stateful_widget(
                    List::new(items)
                        .highlight_style(palette.selected)
                        .highlight_symbol("> "),
                    list_area,
                    &mut state,
                );
            }
        }
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let message = match &self.notice {
            Some(Ok(message)) => message.text(self.language),
            Some(Err(message)) => message.text(self.language),
            None => text::t(self.language, Id::Ready).to_string(),
        };
        let style = if matches!(self.notice, Some(Err(_))) {
            palette.error
        } else {
            palette.fg
        };
        let hint = if self.confirm.is_some() {
            "enter  esc"
        } else if self.settings {
            "↑↓ enter esc"
        } else if self.typing.is_some() {
            "enter  esc"
        } else if self.pane == Pane::Library {
            "/  r  h  s  q     ↑↓ enter"
        } else {
            "esc  tab  space  u  x  l  /  r  s     ↑↓"
        };
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
        frame.render_widget(Paragraph::new(message).style(style), rows[0]);
        frame.render_widget(Paragraph::new(hint).style(palette.dim), rows[1]);
    }

    fn draw_settings(&self, frame: &mut Frame, area: Rect) {
        let palette = palette(self.theme);
        let popup = centered(area, 46, 18);
        frame.render_widget(Clear, popup);
        let block = bordered(text::t(self.language, Id::Settings), true, &palette);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let themes = [
            (Theme::System, Id::ThemeSystem),
            (Theme::Dark, Id::ThemeDark),
            (Theme::Light, Id::ThemeLight),
        ];
        let mut lines = vec![Line::from(text::t(self.language, Id::Appearance)).style(palette.dim)];
        for (index, (theme, id)) in themes.iter().enumerate() {
            lines.push(settings_row(
                self.settings_index == index,
                self.theme == *theme,
                text::t(self.language, *id),
                &palette,
            ));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(text::t(self.language, Id::Language)).style(palette.dim));
        for (index, language) in Language::ALL.iter().enumerate() {
            let row = index + 3;
            lines.push(settings_row(
                self.settings_index == row,
                self.language == *language,
                language.native_name(),
                &palette,
            ));
        }

        let cursor = if self.settings_index < 3 {
            self.settings_index + 1
        } else {
            self.settings_index + 3
        };
        let height = inner.height as usize;
        let offset = cursor.saturating_sub(height.saturating_sub(1));
        let visible: Vec<Line> = lines.into_iter().skip(offset).collect();
        frame.render_widget(Paragraph::new(visible), inner);
    }

    fn draw_confirm(&self, frame: &mut Frame, area: Rect) {
        let Some(confirm) = self.confirm else {
            return;
        };
        let palette = palette(self.theme);
        let popup = centered(area, 64, 7);
        frame.render_widget(Clear, popup);
        let block = bordered(text::t(self.language, Id::Confirm), true, &palette);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let question = match confirm {
            Confirm::UnlockAll => text::t(self.language, Id::UnlockPrompt),
            Confirm::Reset => text::t(self.language, Id::ResetPrompt),
        };
        let body = format!(
            "{question}\n\n{}    {}",
            text::t(self.language, Id::Continue),
            text::t(self.language, Id::Cancel)
        );
        frame.render_widget(
            Paragraph::new(body)
                .style(palette.fg)
                .wrap(Wrap { trim: true }),
            inner,
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
}

fn palette(theme: Theme) -> Palette {
    let (fg, dim, border, selected_bg, selected_fg) = match theme {
        Theme::Light => (
            Color::Black,
            Color::Rgb(90, 90, 90),
            Color::Rgb(180, 180, 180),
            Color::Rgb(224, 164, 52),
            Color::Black,
        ),
        Theme::Dark => (
            Color::White,
            Color::Rgb(160, 160, 160),
            Color::Rgb(70, 70, 70),
            Color::Rgb(224, 164, 52),
            Color::Black,
        ),
        Theme::System => (
            Color::Reset,
            Color::DarkGray,
            Color::DarkGray,
            Color::Rgb(224, 164, 52),
            Color::Black,
        ),
    };
    Palette {
        fg: Style::default().fg(fg),
        dim: Style::default().fg(dim),
        error: Style::default().fg(Color::Rgb(255, 140, 140)),
        selected: Style::default()
            .fg(selected_fg)
            .bg(selected_bg)
            .add_modifier(Modifier::BOLD),
        border: Style::default().fg(border),
        focus: Style::default().fg(Color::Rgb(224, 164, 52)),
    }
}

fn library_block(title: &str, focused: bool, palette: &Palette) -> Block<'static> {
    bordered(title, focused, palette).title(Line::from(" ⚙").right_aligned())
}

fn settings_row(cursor: bool, selected: bool, label: &str, palette: &Palette) -> Line<'static> {
    let mark = if selected { "[x]" } else { "[ ]" };
    let pointer = if cursor { ">" } else { " " };
    let style = if cursor { palette.selected } else { palette.fg };
    Line::from(format!("{pointer} {mark} {label}")).style(style)
}

fn bordered(title: &str, focused: bool, palette: &Palette) -> Block<'static> {
    let title = title.to_string();
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if focused { palette.focus } else { palette.border })
}

fn list_area(
    frame: &mut Frame,
    area: Rect,
    loading: bool,
    language: Language,
    palette: &Palette,
) -> Rect {
    if !loading {
        return area;
    }
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    frame.render_widget(
        Paragraph::new(text::t(language, Id::TalkingToSteam)).style(palette.dim),
        rows[0],
    );
    rows[1]
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
