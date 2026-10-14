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

use crate::backend::app_lister::{AppLister, AppModel};
use crate::backend::app_manager::AppManager;
use crate::backend::connected_steam::ConnectedSteam;
use crate::backend::stat_definitions::{AchievementInfo, StatInfo};
use crate::text::Id;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

pub enum Job {
    Shutdown,
    LoadApps,
    Open(u32),
    Reload(u32),
    SetAchievement {
        app_id: u32,
        id: String,
        unlocked: bool,
    },
    UnlockAll(u32),
    Reset(u32),
    SaveStat {
        app_id: u32,
        id: String,
        value: StatValue,
    },
}

pub enum StatValue {
    Int(i32),
    Float(f32),
}

pub enum UiEvent {
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
    Notice(Result<String, String>),
}

pub fn spawn() -> (Sender<Job>, Receiver<UiEvent>, JoinHandle<()>) {
    let (job_tx, job_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let worker = thread::spawn(move || run(job_rx, event_tx));
    (job_tx, event_rx, worker)
}

fn run(jobs: Receiver<Job>, events: Sender<UiEvent>) {
    while let Ok(job) = jobs.recv() {
        let event = match job {
            Job::Shutdown => break,
            Job::LoadApps => UiEvent::Apps(load_apps()),
            Job::Open(app_id) | Job::Reload(app_id) => load_details(app_id, None),
            Job::SetAchievement {
                app_id,
                id,
                unlocked,
            } => set_achievement(app_id, id, unlocked),
            Job::UnlockAll(app_id) => unlock_all(app_id),
            Job::Reset(app_id) => reset(app_id),
            Job::SaveStat { app_id, id, value } => save_stat(app_id, id, value),
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn load_apps() -> Result<Vec<AppModel>, String> {
    let steam = ConnectedSteam::new(true).map_err(|err| err.to_string())?;
    let lister = AppLister::new(&steam.apps_001, &steam.apps);
    lister.get_owned_apps().map_err(|err| err.to_string())
}

fn load_details(app_id: u32, message: Option<Id>) -> UiEvent {
    let mut manager = match AppManager::new_connected(app_id) {
        Ok(manager) => manager,
        Err(err) => {
            return UiEvent::Notice(Err(err.to_string()));
        }
    };
    UiEvent::Details {
        app_id,
        achievements: manager
            .get_achievements(true)
            .map_err(|err| err.to_string()),
        stats: manager.get_statistics().map_err(|err| err.to_string()),
        message,
    }
}

fn set_achievement(app_id: u32, id: String, unlocked: bool) -> UiEvent {
    let mut manager = match AppManager::new_connected(app_id) {
        Ok(manager) => manager,
        Err(err) => return UiEvent::Notice(Err(err.to_string())),
    };
    if let Err(err) = manager.set_achievement(&id, unlocked, true) {
        return UiEvent::Notice(Err(err.to_string()));
    }
    match manager.get_achievements(true) {
        Ok(achievements) => UiEvent::Achievements {
            app_id,
            achievements: Ok(achievements),
            message: Id::AchievementUpdated,
        },
        Err(err) => UiEvent::Notice(Err(err.to_string())),
    }
}

fn unlock_all(app_id: u32) -> UiEvent {
    let mut manager = match AppManager::new_connected(app_id) {
        Ok(manager) => manager,
        Err(err) => return UiEvent::Notice(Err(err.to_string())),
    };
    if let Err(err) = manager.unlock_all_achievements() {
        return UiEvent::Notice(Err(err.to_string()));
    }
    match manager.get_achievements(true) {
        Ok(achievements) => UiEvent::Achievements {
            app_id,
            achievements: Ok(achievements),
            message: Id::AchievementsUnlocked,
        },
        Err(err) => UiEvent::Notice(Err(err.to_string())),
    }
}

fn reset(app_id: u32) -> UiEvent {
    let manager = match AppManager::new_connected(app_id) {
        Ok(manager) => manager,
        Err(err) => return UiEvent::Notice(Err(err.to_string())),
    };
    if let Err(err) = manager.reset_all_stats(true) {
        return UiEvent::Notice(Err(err.to_string()));
    }
    load_details(app_id, Some(Id::StatsReset))
}

fn save_stat(app_id: u32, id: String, value: StatValue) -> UiEvent {
    let mut manager = match AppManager::new_connected(app_id) {
        Ok(manager) => manager,
        Err(err) => return UiEvent::Notice(Err(err.to_string())),
    };
    let saved = match value {
        StatValue::Int(value) => manager.set_stat_i32(&id, value),
        StatValue::Float(value) => manager.set_stat_f32(&id, value),
    };
    if let Err(err) = saved {
        return UiEvent::Notice(Err(err.to_string()));
    }
    match manager.get_statistics() {
        Ok(stats) => UiEvent::Stats {
            app_id,
            stats: Ok(stats),
            message: Id::StatSaved,
        },
        Err(err) => UiEvent::Notice(Err(err.to_string())),
    }
}
