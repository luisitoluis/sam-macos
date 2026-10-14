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
use crate::text::{self, Id, Language, Message};
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
            cc.egui_ctx.set_theme(egui::ThemePreference::Light);
            install_look(&cc.egui_ctx);
            install_onest(&cc.egui_ctx);
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

/// Onest variable subsets, Latin first. Later faces cover glyphs the earlier
/// ones do not. Default egui faces stay after these as a fallback.
///
/// Onest has no Chinese, Japanese, or Korean. Those come from macOS system
/// faces appended after the subsets.
const ONEST_FACES: &[(&str, &[u8])] = &[
    (
        "Onest Latin",
        include_bytes!("../../assets/fonts/onest-latin-wght-normal.ttf"),
    ),
    (
        "Onest Latin Ext",
        include_bytes!("../../assets/fonts/onest-latin-ext-wght-normal.ttf"),
    ),
    (
        "Onest Cyrillic",
        include_bytes!("../../assets/fonts/onest-cyrillic-wght-normal.ttf"),
    ),
    (
        "Onest Cyrillic Ext",
        include_bytes!("../../assets/fonts/onest-cyrillic-ext-wght-normal.ttf"),
    ),
    (
        "Onest Vietnamese",
        include_bytes!("../../assets/fonts/onest-vietnamese-wght-normal.ttf"),
    ),
    (
        "Onest Symbols",
        include_bytes!("../../assets/fonts/onest-symbols-wght-normal.ttf"),
    ),
    (
        "Onest Math",
        include_bytes!("../../assets/fonts/onest-math-wght-normal.ttf"),
    ),
];

/// Simplified Chinese and Japanese, then Korean. Face 0 is the regular weight.
const CJK_SYSTEM_FACES: &[(&str, &str, u32)] = &[
    (
        "Hiragino Sans GB",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        0,
    ),
    (
        "Apple SD Gothic Neo",
        "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        0,
    ),
];

fn install_onest(ctx: &egui::Context) {
    ctx.set_fonts(gui_font_definitions());
}

fn install_look(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(theme, |style| {
            tune_spacing(style);
            tune_visuals(&mut style.visuals, theme == egui::Theme::Dark);
        });
    }
}

fn tune_spacing(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size = egui::vec2(40.0, 34.0);
    style.spacing.window_margin = egui::Margin::same(16);
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 9.0;
    style.spacing.icon_spacing = 8.0;
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.5));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(22.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
}

fn tune_visuals(visuals: &mut egui::Visuals, dark: bool) {
    let (ink, muted, hairline, canvas, card, field, accent, accent_soft, button, button_hover) =
        if dark {
            (
                egui::Color32::from_rgb(236, 238, 242),
                egui::Color32::from_rgb(156, 163, 175),
                egui::Color32::from_rgb(52, 56, 66),
                egui::Color32::from_rgb(17, 19, 23),
                egui::Color32::from_rgb(28, 31, 37),
                egui::Color32::from_rgb(22, 24, 29),
                egui::Color32::from_rgb(143, 178, 255),
                egui::Color32::from_rgb(36, 48, 74),
                egui::Color32::from_rgb(36, 40, 48),
                egui::Color32::from_rgb(48, 53, 64),
            )
        } else {
            (
                egui::Color32::from_rgb(28, 32, 38),
                egui::Color32::from_rgb(107, 114, 128),
                egui::Color32::from_rgb(226, 229, 234),
                egui::Color32::from_rgb(245, 246, 248),
                egui::Color32::WHITE,
                egui::Color32::from_rgb(247, 248, 250),
                egui::Color32::from_rgb(37, 99, 235),
                egui::Color32::from_rgb(232, 240, 254),
                egui::Color32::WHITE,
                egui::Color32::from_rgb(243, 245, 248),
            )
        };
    let radius = egui::CornerRadius::same(10);
    let hair = egui::Stroke::new(1.0, hairline);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = radius;
        widget.expansion = 0.0;
    }

    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.noninteractive.bg_stroke = hair;
    visuals.widgets.noninteractive.bg_fill = field;
    visuals.widgets.noninteractive.weak_bg_fill = canvas;

    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.inactive.bg_stroke = hair;
    visuals.widgets.inactive.bg_fill = field;
    visuals.widgets.inactive.weak_bg_fill = button;

    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, ink);
    visuals.widgets.hovered.bg_stroke = hair;
    visuals.widgets.hovered.bg_fill = button_hover;
    visuals.widgets.hovered.weak_bg_fill = button_hover;

    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, accent);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.active.bg_fill = accent_soft;
    visuals.widgets.active.weak_bg_fill = accent_soft;
    visuals.widgets.open = visuals.widgets.hovered;

    visuals.selection.bg_fill = accent_soft;
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);
    visuals.hyperlink_color = accent;
    visuals.faint_bg_color = field;
    visuals.extreme_bg_color = field;
    visuals.text_edit_bg_color = Some(field);
    visuals.code_bg_color = field;
    visuals.warn_fg_color = egui::Color32::from_rgb(180, 110, 20);
    visuals.error_fg_color = if dark {
        egui::Color32::from_rgb(255, 160, 160)
    } else {
        egui::Color32::from_rgb(185, 48, 54)
    };
    visuals.override_text_color = Some(ink);
    visuals.weak_text_color = Some(muted);
    visuals.window_corner_radius = egui::CornerRadius::same(16);
    visuals.menu_corner_radius = egui::CornerRadius::same(12);
    visuals.window_fill = card;
    visuals.window_stroke = hair;
    visuals.window_shadow = egui::Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: egui::Color32::from_black_alpha(if dark { 90 } else { 22 }),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 18,
        spread: 0,
        color: egui::Color32::from_black_alpha(if dark { 80 } else { 18 }),
    };
    visuals.panel_fill = canvas;
    visuals.window_highlight_topmost = false;
    visuals.indent_has_left_vline = false;
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
}

fn gui_font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let mut names = Vec::new();
    for (name, bytes) in ONEST_FACES {
        fonts.font_data.insert(
            (*name).to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
        names.push((*name).to_owned());
    }
    for (name, path, index) in CJK_SYSTEM_FACES {
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("{name} was not found; some languages may not render");
            continue;
        };
        let mut data = egui::FontData::from_owned(bytes);
        data.index = *index;
        fonts
            .font_data
            .insert((*name).to_owned(), std::sync::Arc::new(data));
        names.push((*name).to_owned());
    }
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        for (offset, name) in names.iter().enumerate() {
            list.insert(offset, name.clone());
        }
    }
    fonts
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
        message: Option<Id>,
    },
    Achievements {
        app_id: u32,
        achievements: Result<Vec<AchievementInfo>, String>,
        message: Id,
    },
    Stats {
        app_id: u32,
        stats: Result<Vec<StatInfo>, String>,
        message: Id,
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
    kind: AppModelType,
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
    notice: Option<Result<Message, Message>>,
    confirm: Option<ConfirmAction>,
    auto_open: Option<u32>,
    theme: egui::ThemePreference,
    language: Language,
    settings_open: bool,
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
            notice: Some(Ok(Message::Phrase(Id::LoadingLibrary))),
            confirm: None,
            auto_open,
            theme: egui::ThemePreference::Light,
            language: Language::from_locale(),
            settings_open: false,
        };
        app.send(Job::LoadApps);
        app
    }

    fn send(&mut self, job: Job) {
        self.inflight += 1;
        if self.jobs.send(job).is_err() {
            self.inflight = self.inflight.saturating_sub(1);
            self.notice = Some(Err(Message::Phrase(Id::WorkerStopped)));
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
                    self.notice = Some(Ok(Message::Games(count)));
                    if let Some(app_id) = self.auto_open.take() {
                        self.open_app(app_id);
                    }
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
                            game.stat_drafts = drafts_from(&list);
                            game.stats = list;
                            self.notice = Some(Ok(Message::Phrase(message)));
                        }
                        Err(err) => self.notice = Some(Err(Message::Own(err))),
                    }
                }
            }
            UiEvent::Closed => {}
            UiEvent::Notice(notice) => {
                if let Some(game) = self.game.as_mut() {
                    game.loading = false;
                }
                self.notice = Some(match notice {
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
                game.stat_drafts = drafts_from(&list);
                game.stats = list;
            }
            Err(err) => error = Some(err),
        }
        let loaded_name = game.name.clone();
        self.notice = Some(match (error, message) {
            (Some(err), _) => Err(Message::Own(err)),
            (None, Some(message)) => Ok(Message::Phrase(message)),
            (None, None) => Ok(Message::Loaded(loaded_name)),
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
                .unwrap_or_else(|| format!("{} {app_id}", text::t(self.language, Id::AppWord))),
            developer: app.map(|app| app.developer.clone()).unwrap_or_default(),
            kind: app.map(|app| app.app_type).unwrap_or(AppModelType::App),
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
            Ok(_) => self.notice = Some(Ok(Message::Phrase(Id::OpeningGame))),
            Err(err) => self.notice = Some(Err(Message::OpenFailed(err.to_string()))),
        }
    }

    fn library_header(&mut self, ui: &mut egui::Ui) {
        let language = self.language;
        ui.horizontal(|ui| {
            ui.heading(text::t(language, Id::Library));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button_size = egui::Vec2::splat(ui.spacing().interact_size.y);
                if ui
                    .add(egui::Button::new("⚙").frame(false).min_size(button_size))
                    .on_hover_text(text::t(language, Id::Settings))
                    .clicked()
                {
                    self.settings_open = true;
                }
            });
        });
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.settings_open;
        let mut theme = self.theme;
        let mut language = self.language;
        egui::Window::new(text::t(language, Id::Settings))
            .id(egui::Id::new("settings-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                ui.weak(text::t(language, Id::Appearance));
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut theme,
                        egui::ThemePreference::System,
                        text::t(language, Id::ThemeSystem),
                    );
                    ui.selectable_value(
                        &mut theme,
                        egui::ThemePreference::Dark,
                        text::t(language, Id::ThemeDark),
                    );
                    ui.selectable_value(
                        &mut theme,
                        egui::ThemePreference::Light,
                        text::t(language, Id::ThemeLight),
                    );
                });
                ui.add_space(12.0);
                ui.weak(text::t(language, Id::Language));
                egui::ComboBox::from_id_salt("settings-language")
                    .width(ui.available_width())
                    .selected_text(language.native_name())
                    .show_ui(ui, |ui| {
                        for option in Language::ALL {
                            ui.selectable_value(&mut language, option, option.native_name());
                        }
                    });
            });
        if self.theme != theme {
            self.theme = theme;
            ctx.set_theme(theme);
        }
        self.language = language;
        self.settings_open = open;
    }

    fn library_ui(&mut self, ui: &mut egui::Ui) -> Vec<UiAction> {
        let mut actions = Vec::new();
        self.library_header(ui);
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(text::t(self.language, Id::SearchGames))
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.hide_junk, text::t(self.language, Id::HideJunk));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.inflight == 0,
                        egui::Button::new(text::t(self.language, Id::Refresh)),
                    )
                    .clicked()
                {
                    actions.push(UiAction::RefreshApps);
                }
            });
        });

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

        ui.weak(text::shown(self.language, visible.len()));
        if self.apps.is_empty() && self.inflight > 0 {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(text::t(self.language, Id::LoadingLibrary));
            });
        }

        let selected = self.game.as_ref().map(|game| game.app_id);
        let idle = self.inflight == 0;
        let footer_height = ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 2.0;
        let scroll_height = (ui.available_height() - footer_height).max(0.0);
        egui::ScrollArea::vertical()
            .id_salt("library-scroll")
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for index in visible {
                    let app = &self.apps[index];
                    let kind = text::t(self.language, kind_id(app.app_type));
                    let app_id = app.app_id;
                    let name = app.app_name.clone();
                    let meta = format!("{app_id} · {kind}");
                    let response = clickable_card(
                        ui,
                        egui::Id::new(("library-row", app_id)),
                        selected == Some(app_id),
                        |ui| {
                            ui.strong(name);
                            ui.weak(meta);
                        },
                    );
                    if idle && response.clicked() {
                        actions.push(UiAction::Open(app_id));
                    }
                }
            });

        ui.add_space(4.0);
        ui.weak(format!("Sam {}", env!("CARGO_PKG_VERSION")));
        actions
    }

    fn game_ui(&mut self, ui: &mut egui::Ui) -> Vec<UiAction> {
        let busy = self.inflight > 0;
        let Some(game) = self.game.as_mut() else {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.label(
                    egui::RichText::new(text::t(self.language, Id::SelectGame))
                        .size(16.0)
                        .color(ui.visuals().weak_text_color()),
                );
            });
            return Vec::new();
        };
        let language = self.language;
        show_game(ui, game, busy, language)
    }

    fn status_ui(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.inflight > 0 {
                ui.spinner();
            }
            let text = match &self.notice {
                Some(Ok(message)) => Ok(message.text(self.language)),
                Some(Err(message)) => Err(message.text(self.language)),
                None => Ok(text::t(self.language, Id::Ready).to_owned()),
            };
            match text {
                Ok(message) => {
                    ui.weak(message);
                }
                Err(message) => {
                    ui.colored_label(ui.visuals().error_fg_color, message);
                }
            }
        });
    }

    fn confirm_ui(&mut self, ctx: &egui::Context) -> Vec<UiAction> {
        let Some(confirm) = self.confirm else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        let language = self.language;
        egui::Window::new(text::t(language, Id::Confirm))
            .id(egui::Id::new("confirm-window"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(380.0);
                ui.label(match confirm {
                    ConfirmAction::UnlockAll => text::t(language, Id::UnlockPrompt),
                    ConfirmAction::Reset => text::t(language, Id::ResetPrompt),
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(text::t(language, Id::Cancel)).clicked() {
                        self.confirm = None;
                    }
                    if primary_button(ui, true, text::t(language, Id::Continue)).clicked() {
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
        let canvas = ui.visuals().panel_fill;
        let card = ui.visuals().window_fill();
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(canvas)
                    .inner_margin(egui::Margin::symmetric(20, 8)),
            )
            .show(ui, |ui| self.status_ui(ui));
        egui::Panel::left("library")
            .resizable(true)
            .default_size(300.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(card)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui, |ui| actions.extend(self.library_ui(ui)));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(canvas)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ui, |ui| actions.extend(self.game_ui(ui)));
        actions.extend(self.confirm_ui(ui.ctx()));
        self.settings_window(ui.ctx());

        for action in actions {
            self.apply(action);
        }
    }
}

fn show_game(
    ui: &mut egui::Ui,
    game: &mut GameView,
    busy: bool,
    language: Language,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !busy,
                egui::Button::new(text::t(language, Id::BackLibrary)).frame(false),
            )
            .clicked()
        {
            actions.push(UiAction::Close);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if primary_button(ui, true, text::t(language, Id::Launch)).clicked() {
                actions.push(UiAction::Launch);
            }
            if ui
                .add_enabled(!busy, egui::Button::new(text::t(language, Id::Refresh)))
                .clicked()
            {
                actions.push(UiAction::Reload);
            }
        });
    });
    ui.heading(&game.name);
    ui.weak(format!(
        "{} · {} · {}",
        game.app_id,
        text::t(language, kind_id(game.kind)),
        game.developer
    ));

    if game.loading {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.weak(text::t(language, Id::TalkingToSteam));
        });
    }

    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(10)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if segment(
                    ui,
                    game.tab == GameTab::Achievements,
                    text::t(language, Id::Achievements),
                )
                .clicked()
                {
                    game.tab = GameTab::Achievements;
                }
                if segment(ui, game.tab == GameTab::Stats, text::t(language, Id::Stats)).clicked() {
                    game.tab = GameTab::Stats;
                }
            });
        });

    match game.tab {
        GameTab::Achievements => actions.extend(show_achievements(ui, game, busy, language)),
        GameTab::Stats => actions.extend(show_stats(ui, game, busy, language)),
    }
    actions
}

fn show_achievements(
    ui: &mut egui::Ui,
    game: &mut GameView,
    busy: bool,
    language: Language,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    let unlocked = game
        .achievements
        .iter()
        .filter(|achievement| achievement.is_achieved)
        .count();
    ui.horizontal(|ui| {
        ui.label(text::unlocked(language, unlocked, game.achievements.len()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    !busy && !game.loading,
                    egui::Button::new(text::t(language, Id::ResetAll)),
                )
                .clicked()
            {
                actions.push(UiAction::AskReset);
            }
            if ui
                .add_enabled(
                    !busy && !game.loading,
                    egui::Button::new(text::t(language, Id::UnlockAll)),
                )
                .clicked()
            {
                actions.push(UiAction::AskUnlockAll);
            }
        });
    });
    ui.add(
        egui::TextEdit::singleline(&mut game.achievement_query)
            .hint_text(text::t(language, Id::FilterAchievements))
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
                let mut achieved = achievement.is_achieved;
                let id = achievement.id.clone();
                let name = if achievement.name.is_empty() {
                    achievement.id.clone()
                } else {
                    achievement.name.clone()
                };
                let description = achievement.description.clone();
                let percent = achievement.global_achieved_percent;
                card_frame(ui, false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let checkbox = ui.add_enabled(
                            !busy && !protected,
                            egui::Checkbox::new(&mut achieved, ""),
                        );
                        if checkbox.changed() {
                            actions.push(UiAction::SetAchievement {
                                id: id.clone(),
                                unlocked: achieved,
                            });
                        }
                        ui.vertical(|ui| {
                            ui.strong(name);
                            if !description.is_empty() {
                                ui.weak(description);
                            }
                            let mut meta = id;
                            if protected {
                                meta.push_str(" · ");
                                meta.push_str(text::t(language, Id::Protected));
                            }
                            if let Some(percent) = percent {
                                meta.push_str(" · ");
                                meta.push_str(&text::of_players(language, percent));
                            }
                            ui.weak(meta);
                        });
                    });
                });
                achievement.is_achieved = achieved;
            }
        });
    actions
}

fn show_stats(
    ui: &mut egui::Ui,
    game: &mut GameView,
    busy: bool,
    language: Language,
) -> Vec<UiAction> {
    let mut actions = Vec::new();
    ui.add(
        egui::TextEdit::singleline(&mut game.stat_query)
            .hint_text(text::t(language, Id::FilterStats))
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
                let title = if display_name.is_empty() {
                    stat_id.clone()
                } else {
                    display_name
                };
                let mut meta = stat_id.clone();
                if protected {
                    meta.push_str(" · ");
                    meta.push_str(text::t(language, Id::Protected));
                }
                if increment_only {
                    meta.push_str(" · ");
                    meta.push_str(text::t(language, Id::IncrementOnly));
                }
                card_frame(ui, false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let text_width = (ui.available_width() - 210.0).max(120.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(text_width, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.strong(title);
                                ui.weak(meta);
                            },
                        );
                        ui.add_enabled(
                            !protected,
                            egui::TextEdit::singleline(draft).desired_width(120.0),
                        );
                        if ui
                            .add_enabled(
                                !busy && !protected,
                                egui::Button::new(text::t(language, Id::Save)),
                            )
                            .clicked()
                        {
                            actions.push(UiAction::SaveStat(stat_id));
                        }
                    });
                });
            }
        });
    actions
}

fn row_frame(ui: &egui::Ui, selected: bool) -> egui::Frame {
    let visuals = ui.visuals();
    if selected {
        egui::Frame::new()
            .fill(visuals.selection.bg_fill)
            .stroke(egui::Stroke::new(1.0, visuals.selection.stroke.color))
            .corner_radius(10)
            .inner_margin(egui::Margin::symmetric(12, 8))
    } else {
        egui::Frame::new()
            .corner_radius(10)
            .inner_margin(egui::Margin::symmetric(12, 8))
    }
}

fn card_frame(ui: &egui::Ui, selected: bool) -> egui::Frame {
    let visuals = ui.visuals();
    let fill = if selected {
        visuals.selection.bg_fill
    } else {
        visuals.window_fill()
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, visuals.selection.stroke.color)
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 10))
}

fn clickable_card(
    ui: &mut egui::Ui,
    id: egui::Id,
    selected: bool,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let width = (ui.available_width() - if selected { 26.0 } else { 24.0 }).max(0.0);
    let inner = row_frame(ui, selected).show(ui, |ui| {
        ui.set_min_width(width);
        body(ui);
    });
    ui.interact(inner.response.rect, id, egui::Sense::click())
}

fn primary_button(ui: &mut egui::Ui, enabled: bool, label: &str) -> egui::Response {
    let accent = ui.visuals().selection.stroke.color;
    let text = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(16, 18, 22)
    } else {
        egui::Color32::WHITE
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(label).color(text))
            .fill(accent)
            .stroke(egui::Stroke::NONE)
            .corner_radius(10),
    )
}

fn segment(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let visuals = ui.visuals();
    let color = if selected {
        visuals.selection.stroke.color
    } else {
        visuals.weak_text_color()
    };
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(color))
            .fill(if selected {
                visuals.selection.bg_fill
            } else {
                egui::Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::NONE)
            .frame(selected)
            .corner_radius(8),
    )
}

fn kind_id(kind: AppModelType) -> Id {
    match kind {
        AppModelType::App => Id::KindApp,
        AppModelType::Mod => Id::KindMod,
        AppModelType::Demo => Id::KindDemo,
        AppModelType::Junk => Id::KindJunk,
    }
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
                        message: Id::AchievementUpdated,
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
                        message: Id::AchievementsUnlocked,
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
                Ok(_) => load_details(app_id, Some(Id::StatsReset)),
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

fn load_details(app_id: u32, message: Option<Id>) -> UiEvent {
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
            message: Id::StatSaved,
        },
        Err(err) => UiEvent::Notice(Err(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::gui_font_definitions;
    use eframe::egui;

    #[test]
    fn cjk_fallbacks_cover_language_names() {
        let definitions = gui_font_definitions();
        let family = definitions
            .families
            .get(&egui::FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        let mut fonts = egui::text::Fonts::new(egui::epaint::TextOptions::default(), definitions);
        let mut fonts = fonts.with_pixels_per_point(1.0);
        let font_id = egui::FontId::proportional(16.0);
        let missing = fonts.layout_no_wrap("◻".to_owned(), font_id.clone(), egui::Color32::WHITE);
        let missing_uv = missing.rows[0].row.glyphs[0].uv_rect;
        for sample in ["简体中文", "日本語", "한국어"] {
            let galley =
                fonts.layout_no_wrap(sample.to_owned(), font_id.clone(), egui::Color32::WHITE);
            for glyph in galley.rows.iter().flat_map(|row| row.row.glyphs.iter()) {
                assert!(
                    glyph.uv_rect.size.x > 0.0 && glyph.uv_rect.size.y > 0.0,
                    "{sample} has an empty glyph; family {family:?}"
                );
                assert_ne!(
                    glyph.uv_rect, missing_uv,
                    "{} in {sample} used the missing-glyph box; family {family:?}",
                    glyph.chr
                );
            }
        }
    }
}
