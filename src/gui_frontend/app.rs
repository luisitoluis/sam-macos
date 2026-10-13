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

use crate::backend::app_lister::{AppModel, AppModelType};
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::gui_frontend::request::{
    GetAchievements, GetStats, GetSubscribedAppList, LaunchApp, Request, ResetStats,
    SetAchievement, SetFloatStat, SetIntStat, Shutdown, StopApp, StoreStatsAndAchievements,
    UnlockAllAchievements,
};
use eframe::egui;
use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub fn run(auto_open: Option<u32>) -> u8 {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon_256.png"))
        .expect("assets/icon_256.png is not a valid PNG");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([800.0, 520.0])
            .with_icon(icon),
        ..Default::default()
    };

    match eframe::run_native(
        "Sam",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(SamApp::new(auto_open)))
        }),
    ) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("Failed to open the macOS window: {err}");
            1
        }
    }
}

enum Job {
    Shutdown,
    LoadApps,
    OpenApp {
        app_id: u32,
        previous: Option<u32>,
    },
    Close(u32),
    Reload(u32),
    SetAchievement {
        app_id: u32,
        achievement_id: String,
        unlocked: bool,
    },
    UnlockAll(u32),
    Reset(u32),
    SaveStat {
        app_id: u32,
        stat_id: String,
        value: StatValue,
    },
}

enum StatValue {
    Int(i32),
    Float(f32),
}

enum UiEvent {
    Apps(Result<Vec<AppModel>, String>),
    Details {
        app_id: u32,
        achievements: Result<Vec<AchievementInfo>, String>,
        stats: Result<Vec<StatInfo>, String>,
        message: Option<String>,
    },
    Achievements {
        app_id: u32,
        achievements: Result<Vec<AchievementInfo>, String>,
        message: String,
    },
    Stats {
        app_id: u32,
        stats: Result<Vec<StatInfo>, String>,
        message: String,
    },
    Closed,
    Notice(Result<String, String>),
}

enum UiAction {
    RefreshApps,
    Open(u32),
    Close,
    Reload,
    SetAchievement { id: String, unlocked: bool },
    AskUnlockAll,
    AskReset,
    UnlockAll,
    Reset,
    SaveStat(String),
    Launch,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameTab {
    Achievements,
    Stats,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfirmAction {
    UnlockAll,
    Reset,
}

struct GameView {
    app_id: u32,
    name: String,
    developer: String,
    kind: String,
    tab: GameTab,
    loading: bool,
    achievements: Vec<AchievementInfo>,
    stats: Vec<StatInfo>,
    stat_drafts: HashMap<String, String>,
    achievement_query: String,
    stat_query: String,
}

struct SamApp {
    jobs: Sender<Job>,
    events: Receiver<UiEvent>,
    shutdown_done: Receiver<()>,
    worker: Option<JoinHandle<()>>,
    inflight: u32,
    apps: Vec<AppModel>,
    search: String,
    hide_junk: bool,
    game: Option<GameView>,
    notice: Option<Result<String, String>>,
    confirm: Option<ConfirmAction>,
    auto_open: Option<u32>,
}

impl SamApp {
    fn new(auto_open: Option<u32>) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let worker = thread::spawn(move || steam_worker(job_rx, event_tx, shutdown_tx));

        let mut app = Self {
            jobs: job_tx,
            events: event_rx,
            shutdown_done: shutdown_rx,
            worker: Some(worker),
            inflight: 0,
            apps: Vec::new(),
            search: String::new(),
            hide_junk: true,
            game: None,
            notice: Some(Ok("Loading your Steam library…".to_string())),
            confirm: None,
            auto_open,
        };
        app.send(Job::LoadApps);
        app
    }

    fn send(&mut self, job: Job) {
        self.inflight += 1;
        if self.jobs.send(job).is_err() {
            self.inflight = self.inflight.saturating_sub(1);
            self.notice = Some(Err("Steam worker stopped.".to_string()));
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            self.inflight = self.inflight.saturating_sub(1);
            self.handle(event);
        }
        if self.inflight > 0 {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn handle(&mut self, event: UiEvent) {
        match event {
            UiEvent::Apps(result) => match result {
                Ok(mut apps) => {
                    apps.sort_by(|a, b| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()));
                    let count = apps.len();
                    self.apps = apps;
                    self.notice = Some(Ok(format!("{count} games")));
                    if let Some(app_id) = self.auto_open.take() {
                        self.open_app(app_id);
                    }
                }
                Err(err) => self.notice = Some(Err(err)),
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
                            self.notice = Some(Ok(message));
                        }
                        Err(err) => self.notice = Some(Err(err)),
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
                            game.stat_drafts = drafts_from(&list);
                            game.stats = list;
                            self.notice = Some(Ok(message));
                        }
                        Err(err) => self.notice = Some(Err(err)),
                    }
                }
            }
            UiEvent::Closed => {}
            UiEvent::Notice(notice) => {
                if let Some(game) = self.game.as_mut() {
                    game.loading = false;
                }
                self.notice = Some(notice);
            }
        }
    }

    fn apply_details(
        &mut self,
        app_id: u32,
        achievements: Result<Vec<AchievementInfo>, String>,
        stats: Result<Vec<StatInfo>, String>,
        message: Option<String>,
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
                game.stat_drafts = drafts_from(&list);
                game.stats = list;
            }
            Err(err) => error = Some(err),
        }
        self.notice = Some(match (error, message) {
            (Some(err), _) => Err(err),
            (None, Some(message)) => Ok(message),
            (None, None) => Ok(format!("Loaded {}", game.name)),
        });
    }

    fn open_app(&mut self, app_id: u32) {
        if self.game.as_ref().is_some_and(|game| game.app_id == app_id) {
            return;
        }
        let previous = self.game.as_ref().map(|game| game.app_id);
        let app = self.apps.iter().find(|app| app.app_id == app_id);
        self.game = Some(GameView {
            app_id,
            name: app
                .map(|app| app.app_name.clone())
                .unwrap_or_else(|| format!("App {app_id}")),
            developer: app.map(|app| app.developer.clone()).unwrap_or_default(),
            kind: app
                .map(|app| app.app_type.to_string())
                .unwrap_or_else(|| "App".to_string()),
            tab: GameTab::Achievements,
            loading: true,
            achievements: Vec::new(),
            stats: Vec::new(),
            stat_drafts: HashMap::new(),
            achievement_query: String::new(),
            stat_query: String::new(),
        });
        self.confirm = None;
        self.send(Job::OpenApp { app_id, previous });
    }

    fn apply(&mut self, action: UiAction) {
        match action {
            UiAction::RefreshApps => self.send(Job::LoadApps),
            UiAction::Open(app_id) => self.open_app(app_id),
            UiAction::Close => {
                if let Some(game) = self.game.take() {
                    self.confirm = None;
                    self.send(Job::Close(game.app_id));
                }
            }
            UiAction::Reload => {
                if let Some(game) = self.game.as_mut() {
                    let app_id = game.app_id;
                    game.loading = true;
                    self.send(Job::Reload(app_id));
                }
            }
            UiAction::SetAchievement { id, unlocked } => {
                if let Some(game) = self.game.as_ref() {
                    self.send(Job::SetAchievement {
                        app_id: game.app_id,
                        achievement_id: id,
                        unlocked,
                    });
                }
            }
            UiAction::AskUnlockAll => self.confirm = Some(ConfirmAction::UnlockAll),
            UiAction::AskReset => self.confirm = Some(ConfirmAction::Reset),
            UiAction::UnlockAll => {
                if let Some(game) = self.game.as_mut() {
                    let app_id = game.app_id;
                    game.loading = true;
                    self.send(Job::UnlockAll(app_id));
                }
            }
            UiAction::Reset => {
                if let Some(game) = self.game.as_mut() {
                    let app_id = game.app_id;
                    game.loading = true;
                    self.send(Job::Reset(app_id));
                }
            }
            UiAction::SaveStat(stat_id) => self.save_stat(stat_id),
            UiAction::Launch => self.launch_current_game(),
        }
    }

    fn save_stat(&mut self, stat_id: String) {
        let Some(game) = self.game.as_ref() else {
            return;
        };
        let Some(draft) = game.stat_drafts.get(&stat_id) else {
            return;
        };
        let Some(stat) = game.stats.iter().find(|stat| stat.id() == stat_id) else {
            return;
        };
        let value = match stat {
            StatInfo::Integer(_) => match draft.trim().parse::<i32>() {
                Ok(value) => StatValue::Int(value),
                Err(_) => {
                    self.notice = Some(Err(format!("'{draft}' is not an integer")));
                    return;
                }
            },
            StatInfo::Float(_) => match draft.trim().parse::<f32>() {
                Ok(value) => StatValue::Float(value),
                Err(_) => {
                    self.notice = Some(Err(format!("'{draft}' is not a number")));
                    return;
                }
            },
        };
        self.send(Job::SaveStat {
            app_id: game.app_id,
            stat_id,
            value,
        });
    }

    fn launch_current_game(&mut self) {
        let Some(app_id) = self.game.as_ref().map(|game| game.app_id) else {
            return;
        };
        match Command::new("open")
            .arg(format!("steam://run/{app_id}"))
            .spawn()
        {
            Ok(_) => self.notice = Some(Ok("Opening the game in Steam.".to_string())),
            Err(err) => self.notice = Some(Err(format!("Could not open Steam: {err}"))),
        }
    }

    fn library_ui(&mut self, ui: &mut egui::Ui) -> Vec<UiAction> {
        let mut actions = Vec::new();
        ui.heading("Library");
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search games")
                .desired_width(f32::INFINITY),
        );
        ui.checkbox(&mut self.hide_junk, "Hide junk");
        if ui
            .add_enabled(self.inflight == 0, egui::Button::new("Refresh"))
            .clicked()
        {
            actions.push(UiAction::RefreshApps);
        }
        ui.separator();

        let query = self.search.to_lowercase();
        let visible: Vec<usize> = self
            .apps
            .iter()
            .enumerate()
            .filter(|(_, app)| {
                if self.hide_junk && matches!(app.app_type, AppModelType::Junk) {
                    return false;
                }
                if query.is_empty() {
                    return true;
                }
                app.app_name.to_lowercase().contains(&query)
                    || app.app_id.to_string().contains(&query)
                    || app.developer.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect();

        ui.weak(format!("{} shown", visible.len()));
        if self.apps.is_empty() && self.inflight > 0 {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading your Steam library…");
            });
        }

        let selected = self.game.as_ref().map(|game| game.app_id);
        let idle = self.inflight == 0;
        egui::ScrollArea::vertical()
            .id_salt("library-scroll")
            .show(ui, |ui| {
                for index in visible {
                    let app = &self.apps[index];
                    let label = format!("{}\n{} · {}", app.app_name, app.app_id, app.app_type);
                    if ui
                        .add_enabled(
                            idle,
                            egui::Button::selectable(selected == Some(app.app_id), label),
                        )
                        .clicked()
                    {
                        actions.push(UiAction::Open(app.app_id));
                    }
                }
            });

        ui.separator();
        ui.weak(format!("Sam {}", env!("CARGO_PKG_VERSION")));
        actions
    }

    fn game_ui(&mut self, ui: &mut egui::Ui) -> Vec<UiAction> {
        let busy = self.inflight > 0;
        let Some(game) = self.game.as_mut() else {
            ui.vertical_centered(|ui| {
                ui.add_space(96.0);
                ui.label("Select a game to view achievements and stats.");
            });
            return Vec::new();
        };
        show_game(ui, game, busy)
    }

    fn status_ui(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.inflight > 0 {
                ui.spinner();
            }
            match &self.notice {
                Some(Ok(message)) => {
                    ui.label(message);
                }
                Some(Err(message)) => {
                    ui.colored_label(egui::Color32::from_rgb(255, 140, 140), message);
                }
                None => {
                    ui.label("Ready");
                }
            }
        });
    }

    fn confirm_ui(&mut self, ctx: &egui::Context) -> Vec<UiAction> {
        let Some(confirm) = self.confirm else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        egui::Window::new("Confirm")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(match confirm {
                    ConfirmAction::UnlockAll => {
                        "Unlock every unprotected achievement in this game?"
                    }
                    ConfirmAction::Reset => "Reset stats and lock achievements for this game?",
                });
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm = None;
                    }
                    if ui.button("Continue").clicked() {
                        self.confirm = None;
                        actions.push(match confirm {
                            ConfirmAction::UnlockAll => UiAction::UnlockAll,
                            ConfirmAction::Reset => UiAction::Reset,
                        });
                    }
                });
            });
        actions
    }
}

impl Drop for SamApp {
    fn drop(&mut self) {
        let _ = self.jobs.send(Job::Shutdown);
        if self
            .shutdown_done
            .recv_timeout(Duration::from_secs(20))
            .is_ok()
            && let Some(handle) = self.worker.take()
        {
            let _ = handle.join();
        }
    }
}

impl eframe::App for SamApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut actions = Vec::new();
        egui::Panel::bottom("status").show(ui, |ui| self.status_ui(ui));
        egui::Panel::left("library")
            .resizable(true)
            .default_size(320.0)
            .show(ui, |ui| actions.extend(self.library_ui(ui)));
        egui::CentralPanel::default().show(ui, |ui| actions.extend(self.game_ui(ui)));
        actions.extend(self.confirm_ui(ui.ctx()));

        for action in actions {
            self.apply(action);
        }
    }
}

fn show_game(ui: &mut egui::Ui, game: &mut GameView, busy: bool) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new("← Library"))
            .clicked()
        {
            actions.push(UiAction::Close);
        }
        ui.heading(&game.name);
        if ui.button("Launch").clicked() {
            actions.push(UiAction::Launch);
        }
        if ui
            .add_enabled(!busy, egui::Button::new("Refresh"))
            .clicked()
        {
            actions.push(UiAction::Reload);
        }
    });
    ui.weak(format!(
        "{} · {} · {}",
        game.app_id, game.kind, game.developer
    ));

    if game.loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Talking to Steam…");
        });
    }

    ui.horizontal(|ui| {
        ui.selectable_value(&mut game.tab, GameTab::Achievements, "Achievements");
        ui.selectable_value(&mut game.tab, GameTab::Stats, "Stats");
    });
    ui.separator();

    match game.tab {
        GameTab::Achievements => actions.extend(show_achievements(ui, game, busy)),
        GameTab::Stats => actions.extend(show_stats(ui, game, busy)),
    }
    actions
}

fn show_achievements(ui: &mut egui::Ui, game: &mut GameView, busy: bool) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let unlocked = game
        .achievements
        .iter()
        .filter(|achievement| achievement.is_achieved)
        .count();
    ui.horizontal(|ui| {
        ui.label(format!("{unlocked}/{} unlocked", game.achievements.len()));
        if ui
            .add_enabled(!busy && !game.loading, egui::Button::new("Unlock all"))
            .clicked()
        {
            actions.push(UiAction::AskUnlockAll);
        }
        if ui
            .add_enabled(!busy && !game.loading, egui::Button::new("Reset all"))
            .clicked()
        {
            actions.push(UiAction::AskReset);
        }
    });
    ui.add(
        egui::TextEdit::singleline(&mut game.achievement_query)
            .hint_text("Filter achievements")
            .desired_width(f32::INFINITY),
    );

    let query = game.achievement_query.to_lowercase();
    let visible: Vec<usize> = game
        .achievements
        .iter()
        .enumerate()
        .filter(|(_, achievement)| {
            query.is_empty()
                || achievement.name.to_lowercase().contains(&query)
                || achievement.id.to_lowercase().contains(&query)
                || achievement.description.to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("achievements-scroll")
        .show(ui, |ui| {
            for index in visible {
                let achievement = &mut game.achievements[index];
                let protected = achievement.permission != 0;
                ui.horizontal(|ui| {
                    let mut achieved = achievement.is_achieved;
                    let checkbox =
                        ui.add_enabled(!busy && !protected, egui::Checkbox::new(&mut achieved, ""));
                    if checkbox.changed() {
                        achievement.is_achieved = achieved;
                        actions.push(UiAction::SetAchievement {
                            id: achievement.id.clone(),
                            unlocked: achieved,
                        });
                    }
                    ui.vertical(|ui| {
                        ui.strong(if achievement.name.is_empty() {
                            achievement.id.as_str()
                        } else {
                            achievement.name.as_str()
                        });
                        if !achievement.description.is_empty() {
                            ui.weak(&achievement.description);
                        }
                        let mut meta = achievement.id.clone();
                        if protected {
                            meta.push_str(" · protected");
                        }
                        if let Some(percent) = achievement.global_achieved_percent {
                            meta.push_str(&format!(" · {percent:.1}% of players"));
                        }
                        ui.weak(meta);
                    });
                });
                ui.separator();
            }
        });
    actions
}

fn show_stats(ui: &mut egui::Ui, game: &mut GameView, busy: bool) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.add(
        egui::TextEdit::singleline(&mut game.stat_query)
            .hint_text("Filter stats")
            .desired_width(f32::INFINITY),
    );

    let query = game.stat_query.to_lowercase();
    let visible: Vec<usize> = game
        .stats
        .iter()
        .enumerate()
        .filter(|(_, stat)| {
            query.is_empty()
                || stat.id().to_lowercase().contains(&query)
                || stat.display_name().to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("stats-scroll")
        .show(ui, |ui| {
            for index in visible {
                let stat_id = game.stats[index].id().to_string();
                let display_name = game.stats[index].display_name().to_string();
                let protected = (game.stats[index].permission() & 2) != 0;
                let increment_only = match &game.stats[index] {
                    StatInfo::Integer(stat) => stat.is_increment_only,
                    StatInfo::Float(stat) => stat.is_increment_only,
                };
                let draft = game.stat_drafts.entry(stat_id.clone()).or_default();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(if display_name.is_empty() {
                            stat_id.as_str()
                        } else {
                            display_name.as_str()
                        });
                        let mut meta = stat_id.clone();
                        if protected {
                            meta.push_str(" · protected");
                        }
                        if increment_only {
                            meta.push_str(" · increment only");
                        }
                        ui.weak(meta);
                    });
                    ui.add_enabled(
                        !protected,
                        egui::TextEdit::singleline(draft).desired_width(120.0),
                    );
                    if ui
                        .add_enabled(!busy && !protected, egui::Button::new("Save"))
                        .clicked()
                    {
                        actions.push(UiAction::SaveStat(stat_id));
                    }
                });
                ui.separator();
            }
        });
    actions
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

fn steam_worker(jobs: Receiver<Job>, events: Sender<UiEvent>, shutdown_done: Sender<()>) {
    while let Ok(job) = jobs.recv() {
        let event = match job {
            Job::Shutdown => {
                let _ = Shutdown.request();
                let _ = shutdown_done.send(());
                break;
            }
            Job::LoadApps => UiEvent::Apps(
                GetSubscribedAppList
                    .request()
                    .map_err(|err| err.to_string()),
            ),
            Job::OpenApp { app_id, previous } => open_app(app_id, previous),
            Job::Close(app_id) => {
                let _ = (StopApp { app_id }).request();
                UiEvent::Closed
            }
            Job::Reload(app_id) => load_details(app_id, None),
            Job::SetAchievement {
                app_id,
                achievement_id,
                unlocked,
            } => match (SetAchievement {
                app_id,
                achievement_id,
                unlocked,
                store: true,
            }
            .request())
            {
                Ok(_) => match (GetAchievements { app_id }.request()) {
                    Ok(achievements) => UiEvent::Achievements {
                        app_id,
                        achievements: Ok(achievements),
                        message: "Achievement updated.".to_string(),
                    },
                    Err(err) => UiEvent::Notice(Err(err.to_string())),
                },
                Err(err) => UiEvent::Notice(Err(err.to_string())),
            },
            Job::UnlockAll(app_id) => match (UnlockAllAchievements { app_id }.request()) {
                Ok(_) => match (GetAchievements { app_id }.request()) {
                    Ok(achievements) => UiEvent::Achievements {
                        app_id,
                        achievements: Ok(achievements),
                        message: "Achievements unlocked.".to_string(),
                    },
                    Err(err) => UiEvent::Notice(Err(err.to_string())),
                },
                Err(err) => UiEvent::Notice(Err(err.to_string())),
            },
            Job::Reset(app_id) => match (ResetStats {
                app_id,
                achievements_too: true,
            }
            .request())
            {
                Ok(_) => load_details(app_id, Some("Stats and achievements reset.".to_string())),
                Err(err) => UiEvent::Notice(Err(err.to_string())),
            },
            Job::SaveStat {
                app_id,
                stat_id,
                value,
            } => save_stat(app_id, stat_id, value),
        };

        if events.send(event).is_err() {
            break;
        }
    }
}

fn open_app(app_id: u32, previous: Option<u32>) -> UiEvent {
    if let Some(previous) = previous
        && previous != app_id
        && let Err(err) = (StopApp { app_id: previous }).request()
    {
        return UiEvent::Notice(Err(err.to_string()));
    }

    if let Err(err) = (LaunchApp { app_id }).request() {
        return UiEvent::Notice(Err(err.to_string()));
    }
    load_details(app_id, None)
}

fn load_details(app_id: u32, message: Option<String>) -> UiEvent {
    UiEvent::Details {
        app_id,
        achievements: GetAchievements { app_id }
            .request()
            .map_err(|err| err.to_string()),
        stats: GetStats { app_id }.request().map_err(|err| err.to_string()),
        message,
    }
}

fn save_stat(app_id: u32, stat_id: String, value: StatValue) -> UiEvent {
    let saved = match value {
        StatValue::Int(value) => SetIntStat {
            app_id,
            stat_id,
            value,
        }
        .request(),
        StatValue::Float(value) => SetFloatStat {
            app_id,
            stat_id,
            value,
        }
        .request(),
    };

    if let Err(err) = saved {
        return UiEvent::Notice(Err(err.to_string()));
    }
    if let Err(err) = (StoreStatsAndAchievements { app_id }).request() {
        return UiEvent::Notice(Err(err.to_string()));
    }

    match (GetStats { app_id }.request()) {
        Ok(stats) => UiEvent::Stats {
            app_id,
            stats: Ok(stats),
            message: "Stat saved.".to_string(),
        },
        Err(err) => UiEvent::Notice(Err(err.to_string())),
    }
}
