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

use crate::dev_println;
use crate::steam_client::steam_apps_001_wrapper::{SteamApps001, SteamApps001AppDataKeys};
use crate::steam_client::steam_apps_wrapper::SteamApps;
use crate::steam_client::steamworks_types::AppId_t;
use crate::utils::app_paths::get_app_cache_dir;
use crate::utils::ipc_types::SamError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime};

pub struct AppLister<'a> {
    app_list_url: String,
    app_list_local: String,
    current_language: String,
    steam_apps_001: &'a SteamApps001,
    steam_apps: &'a SteamApps,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppModel {
    pub app_id: AppId_t,
    pub app_name: String,
    pub image_url: Option<String>,
    pub app_type: AppModelType,
    pub developer: String,
    pub metacritic_score: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum AppModelType {
    App,
    Mod,
    Demo,
    Junk,
}

impl Display for AppModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppModelType::App => write!(f, "App"),
            AppModelType::Mod => write!(f, "Mod"),
            AppModelType::Demo => write!(f, "Demo"),
            AppModelType::Junk => write!(f, "Junk"),
        }
    }
}

impl FromStr for AppModelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "app" => Ok(AppModelType::App),
            "mod" => Ok(AppModelType::Mod),
            "demo" => Ok(AppModelType::Demo),
            "junk" => Ok(AppModelType::Junk),
            _ => Err(format!("'{}' is not a valid AppModelType", s)),
        }
    }
}

#[derive(Deserialize)]
pub struct XmlGame {
    #[serde(rename = "$text")]
    pub app_id: u32,
    #[serde(rename = "@type")]
    pub app_type: Option<String>,
}

#[derive(Deserialize)]
struct XmlGames {
    #[serde(rename = "game")]
    pub games: Vec<XmlGame>,
}
impl<'a> AppLister<'a> {
    pub fn new(steam_apps_001: &'a SteamApps001, steam_apps: &'a SteamApps) -> Self {
        let cache_dir = get_app_cache_dir();
        let app_list_url = std::env::var("SAM_APP_LIST_URL")
            .unwrap_or(String::from("https://gib.me/sam/games.xml"));
        let app_list_local = std::env::var("APP_LIST_LOCAL").unwrap_or(String::from("/apps.xml"));
        let current_language = steam_apps.get_current_game_language();

        AppLister {
            app_list_url,
            app_list_local: cache_dir + &app_list_local,
            current_language,
            steam_apps_001,
            steam_apps,
        }
    }

    fn load_app_list_file(&self) -> Result<XmlGames, SamError> {
        let f = File::open(&self.app_list_local).map_err(|_| SamError::AppListRetrievalFailed)?;
        let f = BufReader::new(f);
        let xml_data: XmlGames =
            quick_xml::de::from_reader(f).map_err(|_| SamError::AppListRetrievalFailed)?;
        Ok(xml_data)
    }

    /// A cached list is always used right away, even when it is old: the
    /// download can take close to a minute. Old lists are refreshed in the
    /// background for the next load.
    fn get_xml_games(&self) -> Result<XmlGames, SamError> {
        let age = fs::metadata(&self.app_list_local)
            .and_then(|metadata| metadata.modified())
            .ok()
            .map(|modified| SystemTime::now().duration_since(modified).unwrap_or_default());

        if let Some(age) = age {
            match self.load_app_list_file() {
                Ok(xml_games) => {
                    dev_println!("[ORCHESTRATOR] Loading app list from local location");
                    if age > APP_LIST_MAX_AGE {
                        refresh_app_list_in_background(
                            self.app_list_url.clone(),
                            self.app_list_local.clone(),
                        );
                    }
                    return Ok(xml_games);
                }
                Err(_) => dev_println!("[ORCHESTRATOR] Cached app list is unreadable"),
            }
        }

        let (source, xml_games) = download_app_list(&self.app_list_url)?;
        store_app_list(&self.app_list_local, &source);
        Ok(xml_games)
    }

    fn get_app_image_url(&self, app_id: &AppId_t) -> Option<String> {
        let candidate = self
            .steam_apps_001
            .get_app_data(
                app_id,
                &SteamApps001AppDataKeys::SmallCapsule(&self.current_language).as_string(),
            )
            .unwrap_or("".to_owned());
        if !candidate.is_empty() {
            return Some(format!(
                "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"
            ));
        }

        if self.current_language != "english" {
            let candidate = self
                .steam_apps_001
                .get_app_data(
                    app_id,
                    &SteamApps001AppDataKeys::SmallCapsule("english").as_string(),
                )
                .unwrap_or("".to_owned());
            if !candidate.is_empty() {
                return Some(format!(
                    "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app_id}/{candidate}"
                ));
            }
        }

        let candidate = self
            .steam_apps_001
            .get_app_data(app_id, &SteamApps001AppDataKeys::Logo.as_string())
            .unwrap_or("".to_owned());
        if !candidate.is_empty() {
            return Some(format!(
                "https://cdn.steamstatic.com/steamcommunity/public/images/apps/{app_id}/{candidate}.jpg"
            ));
        }

        dev_println!("[ORCHESTRATOR] Failed to find image for app {}", app_id);

        None
    }

    pub fn get_app(&self, app_id: AppId_t, xml_game: &XmlGame) -> Result<AppModel, SamError> {
        let app_name = self
            .steam_apps_001
            .get_app_data(&app_id, &SteamApps001AppDataKeys::Name.as_string())
            .map_err(|_| SamError::AppListRetrievalFailed)?;
        let developer = self
            .steam_apps_001
            .get_app_data(&app_id, &SteamApps001AppDataKeys::Developer.as_string())
            .unwrap_or("Unknown".to_string());
        let metacritic_score: Option<u8> = self
            .steam_apps_001
            .get_app_data(
                &app_id,
                &SteamApps001AppDataKeys::MetacriticScore.as_string(),
            )
            .ok()
            .and_then(|s| s.parse().ok());
        let image_url = self.get_app_image_url(&app_id);

        Ok(AppModel {
            app_id,
            app_name,
            image_url,
            app_type: if xml_game.app_type.as_ref().is_none() {
                AppModelType::App
            } else {
                AppModelType::from_str(xml_game.app_type.as_ref().unwrap())
                    .map_err(|_| SamError::AppListRetrievalFailed)?
            },
            developer,
            metacritic_score,
        })
    }

    pub fn get_owned_apps(&self) -> Result<Vec<AppModel>, SamError> {
        let models = self.scan_owned_apps()?;
        store_owned_apps(&models);
        Ok(models)
    }

    fn scan_owned_apps(&self) -> Result<Vec<AppModel>, SamError> {
        let xml_games = self.get_xml_games()?;

        // IClientUserStats::GetNumAchievedAchievements( 291550, ) = 0,
        // IClientUserStats::GetNumAchievements( 291550, ) = 65

        let mut models = vec![];
        for xml_game in xml_games.games {
            let app_id: AppId_t = xml_game.app_id;

            if !self.steam_apps.is_subscribed_app(app_id).unwrap_or(false) {
                continue;
            }

            match self.get_app(app_id, &xml_game) {
                Ok(app) => models.push(app),
                Err(_) => dev_println!("[ORCHESTRATOR] Skipping app {app_id}: no app data"),
            }
        }

        Ok(models)
    }
}

const APP_LIST_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const OWNED_APPS_FILE: &str = "/owned_apps.json";

static APP_LIST_REFRESHING: AtomicBool = AtomicBool::new(false);

fn download_app_list(url: &str) -> Result<(String, XmlGames), SamError> {
    dev_println!("[ORCHESTRATOR] Downloading app list from:  {url}");
    // The host can take close to a minute; reqwest's default 30 s timeout is too short.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|_| SamError::AppListRetrievalFailed)?;
    let source = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|err| {
            dev_println!("[ORCHESTRATOR] App list download failed: {err}");
            SamError::AppListRetrievalFailed
        })?;
    let xml_games =
        quick_xml::de::from_str(&source).map_err(|_| SamError::AppListRetrievalFailed)?;
    Ok((source, xml_games))
}

fn refresh_app_list_in_background(url: String, local: String) {
    if APP_LIST_REFRESHING.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        if let Ok((source, _)) = download_app_list(&url) {
            store_app_list(&local, &source);
        }
        APP_LIST_REFRESHING.store(false, Ordering::SeqCst);
    });
}

fn store_app_list(local: &str, source: &str) {
    dev_println!("[ORCHESTRATOR] App list loaded. Saving in:  {local}");
    if !write_atomically(local, source.as_bytes()) {
        dev_println!("[ORCHESTRATOR] Could not cache the app list");
    }
}

/// The owned apps found by the last full scan. Frontends show these while a
/// new scan runs, because checking every known app with Steam takes seconds.
pub fn cached_owned_apps() -> Option<Vec<AppModel>> {
    let raw = fs::read(get_app_cache_dir() + OWNED_APPS_FILE).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn store_owned_apps(apps: &[AppModel]) {
    if let Ok(raw) = serde_json::to_vec(apps) {
        write_atomically(&(get_app_cache_dir() + OWNED_APPS_FILE), &raw);
    }
}

/// Another process may be reading the same file, so it never sees half of it.
fn write_atomically(path: &str, bytes: &[u8]) -> bool {
    let temp = format!("{path}.tmp{}", std::process::id());
    if fs::write(&temp, bytes).is_ok() && fs::rename(&temp, path).is_ok() {
        return true;
    }
    let _ = fs::remove_file(&temp);
    false
}
