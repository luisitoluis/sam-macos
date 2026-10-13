use crate::utils::ipc_types::SamError;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

pub struct SteamLocator {
    lib_path: Option<PathBuf>,
    user_game_stats_schema_prefix: Option<String>,
    local_app_banner_file_prefix: Option<String>,
}

impl SteamLocator {
    pub fn new() -> Self {
        Self {
            lib_path: None,
            user_game_stats_schema_prefix: None,
            local_app_banner_file_prefix: None,
        }
    }

    pub fn global() -> &'static RwLock<SteamLocator> {
        static INSTANCE: OnceLock<RwLock<SteamLocator>> = OnceLock::new();
        INSTANCE.get_or_init(|| RwLock::new(SteamLocator::new()))
    }

    pub fn get_lib_path(&mut self, silent: bool) -> Option<PathBuf> {
        if self.lib_path.is_none() {
            self.lib_path = Self::get_steamclient_lib_path(silent);
        }
        self.lib_path.clone()
    }

    pub fn get_user_game_stats_schema(&mut self, app_id: &u32) -> Result<PathBuf, SamError> {
        if self.user_game_stats_schema_prefix.is_none() {
            self.user_game_stats_schema_prefix = Self::get_user_game_stats_schema_prefix();
        }

        if let Some(prefix) = self.user_game_stats_schema_prefix.as_ref() {
            let path_str = format!("{}{}.bin", prefix, app_id);
            return Ok(PathBuf::from(path_str));
        }

        Err(SamError::UnknownError)
    }

    pub fn get_local_app_banner_file_path(&mut self, app_id: &u32) -> Result<PathBuf, SamError> {
        if self.local_app_banner_file_prefix.is_none() {
            self.local_app_banner_file_prefix = Self::get_local_app_banner_file_prefix();
        }

        if let Some(prefix) = self.local_app_banner_file_prefix.as_ref() {
            let path_str = format!("{}{}/header.jpg", prefix, app_id);
            return Ok(PathBuf::from(path_str));
        }

        Err(SamError::UnknownError)
    }

    /// macOS version - tries multiple approaches to find a working steam client library.
    /// Note: There is NO official libsteam_api.dylib for Apple Silicon. We try:
    /// 1. Custom SAM_STEAMCLIENT_PATH env var
    /// 2. Linux steamclient.so via Rosetta 2 (if user has it)
    /// 3. Standard Steam installation paths
    #[cfg(target_os = "macos")]
    pub fn get_steamclient_lib_path(silent: bool) -> Option<PathBuf> {
        use std::path::Path;

        // 1. Environment variable override
        if let Ok(path_str) = std::env::var("SAM_STEAMCLIENT_PATH") {
            return Some(Path::new(&path_str).to_owned());
        }

        // 2. Standard macOS Steam installation paths
        // Try common macOS Steam locations
        let steam_install_paths: Vec<PathBuf> = Self::get_local_steam_install_root_folders()
            .into_iter()
            .flat_map(|path| {
                vec![
                    path.join("linux64/steamclient.so"),  // Linux client via Rosetta
                    path.join("../MacOS/steamclient.so"), // Inside Steam.app (legacy)
                    path.join("steamclient.so"),          // Root level
                ]
            })
            .filter(|path| path.exists())
            .collect();

        if let Some(first_path) = steam_install_paths.first() {
            if !silent {
                eprintln!(
                    "[STEAM LOCATOR] Using Steam client library: {}",
                    first_path.display()
                );
                eprintln!(
                    "[STEAM LOCATOR] Note: If using steamclient.so on Apple Silicon, ensure Rosetta 2 is enabled"
                );
            }
            return Some(first_path.clone());
        }

        if !silent {
            eprintln!("[STEAM LOCATOR] Warning: Could not find steam client library on macOS");
            eprintln!("[STEAM LOCATOR] Expected locations:");
            eprintln!("[STEAM LOCATOR]   - ~/.local/share/Steam/linux64/steamclient.so");
            eprintln!(
                "[STEAM LOCATOR]   - ~/Library/Application Support/Steam/linux64/steamclient.so"
            );
        }

        None
    }

    /// macOS: get user game stats schema prefix
    #[cfg(target_os = "macos")]
    pub fn get_user_game_stats_schema_prefix() -> Option<String> {
        let dirs = Self::get_local_steam_install_root_folders();

        if dirs.is_empty() {
            None
        } else {
            Some(dirs[0].to_str()?.to_owned() + "/appcache/stats/UserGameStatsSchema_")
        }
    }

    /// macOS: get local Steam installation root folders
    #[cfg(target_os = "macos")]
    pub fn get_local_steam_install_root_folders() -> Vec<PathBuf> {
        // Environment variable override for custom Steam location
        if let Ok(path) = std::env::var("SAM_STEAM_INSTALL_ROOT") {
            return vec![PathBuf::from(path)];
        }

        let home = std::env::var("HOME").expect("Failed to get home dir");
        let home_path = PathBuf::from(home);

        // Standard macOS Steam installation locations
        let potential_dirs = [
            home_path.join(".local/share/Steam"),                // Standard
            home_path.join("Library/Application Support/Steam"), // App Store / alternative
        ];

        potential_dirs
            .into_iter()
            .filter(|path| path.exists())
            .collect()
    }

    /// macOS: get local app banner file prefix
    #[cfg(target_os = "macos")]
    pub fn get_local_app_banner_file_prefix() -> Option<String> {
        let dirs = Self::get_local_steam_install_root_folders();

        if dirs.is_empty() {
            None
        } else {
            Some(dirs[0].to_str()?.to_owned() + "/appcache/librarycache/")
        }
    }
}
