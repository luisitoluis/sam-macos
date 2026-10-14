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

use crate::steam_client::steam_client_vtable::{ISteamClient, STEAMCLIENT_INTERFACE_VERSION};
use crate::steam_client::steam_client_wrapper::SteamClient;
use crate::steam_client::steamworks_types::{
    CreateInterfaceFn, SteamFreeLastCallbackFn, SteamGetCallbackFn,
};
use crate::utils::ipc_types::SamError;
use crate::utils::steam_locator::SteamLocator;
use libloading::{Library, Symbol};
use std::os::raw::c_char;
use std::sync::OnceLock;

static STEAM_CLIENT_LIB: OnceLock<Library> = OnceLock::new(); // Make the lifetime 'static

/// macOS version - loads Steam client library.
/// Uses dlopen under the hood via libloading.
/// On Apple Silicon, this will load the Linux steamclient.so via Rosetta 2.
#[cfg(target_os = "macos")]
pub fn load_steamclient_library(silent: bool) -> Result<Library, Box<dyn std::error::Error>> {
    unsafe {
        let steam_locator_lock = SteamLocator::global();
        let mut steam_locator = steam_locator_lock.write()?;
        let steamclient_lib_path = steam_locator
            .get_lib_path(silent)
            .ok_or(SamError::UnknownError)?;

        let lib_steamclient = match Library::new(&steamclient_lib_path) {
            Ok(lib) => lib,
            Err(e) => {
                if !silent {
                    eprintln!(
                        "[STEAM CLIENT] Failed to open library at {} ({})",
                        steamclient_lib_path.display(),
                        e
                    );
                    eprintln!("[STEAM CLIENT] Troubleshooting:");
                    eprintln!(
                        "[STEAM CLIENT]   1. Ensure Steam is installed and has been launched at least once"
                    );
                    eprintln!(
                        "[STEAM CLIENT]   2. Make sure ~/.local/share/Steam/linux64/steamclient.so exists"
                    );
                    eprintln!(
                        "[STEAM CLIENT]   3. If on Apple Silicon, enable Rosetta 2: /usr/sbin/softwareupdate --install-rosetta"
                    );
                }
                return Err(Box::new(e));
            }
        };

        Ok(lib_steamclient)
    }
}

pub fn new_steam_client_interface(
    steamclient_so: &Library,
) -> Result<
    (
        *mut ISteamClient,
        Symbol<'_, SteamGetCallbackFn>,
        Symbol<'_, SteamFreeLastCallbackFn>,
    ),
    Box<dyn std::error::Error>,
> {
    unsafe {
        let create_interface: Symbol<CreateInterfaceFn> = steamclient_so.get(b"CreateInterface")?;
        let steam_get_callback: Symbol<SteamGetCallbackFn> =
            steamclient_so.get(b"Steam_BGetCallback")?;
        let steam_free_last_callback: Symbol<SteamFreeLastCallbackFn> =
            steamclient_so.get(b"Steam_FreeLastCallback")?;

        let mut return_code = 1;
        let client = create_interface(
            STEAMCLIENT_INTERFACE_VERSION.as_ptr() as *const c_char,
            &mut return_code,
        );

        if return_code != 0 {
            return Err(Box::from(format!(
                "Steam client interface creation failed with code {}",
                return_code
            )));
        }

        if client.is_null() {
            return Err(Box::from(
                "Steam client failed to create interface (pointer is NULL)",
            ));
        }

        Ok((client, steam_get_callback, steam_free_last_callback))
    }
}

pub fn create_steam_client(silent: bool) -> Result<SteamClient, Box<dyn std::error::Error>> {
    if STEAM_CLIENT_LIB.get().is_none() {
        let steamclient_so = load_steamclient_library(silent)?;
        match STEAM_CLIENT_LIB.set(steamclient_so) {
            Ok(_) => {}
            Err(_) => panic!("Failed to create steam client"),
        };
    }

    let (raw_client, callback_fn, free_callback_fn) =
        new_steam_client_interface(STEAM_CLIENT_LIB.get().unwrap())?;
    let client = unsafe { SteamClient::from_raw(raw_client, callback_fn, free_callback_fn) };

    Ok(client)
}
