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

mod app;
mod image_loader;
mod request;
mod theme;

use crate::gui_frontend::request::Request;
use crate::utils::bidir_child::BidirChild;
use std::sync::RwLock;

pub(crate) static DEFAULT_PROCESS: RwLock<Option<BidirChild>> = RwLock::new(None);

pub fn main_ui(orchestrator: BidirChild) -> u8 {
    {
        let mut guard = DEFAULT_PROCESS.write().unwrap();
        *guard = Some(orchestrator);
    }

    let exit_code = app::run(crate::utils::arguments::auto_open_app_id());
    if exit_code != 0 {
        let _ = request::Shutdown.request();
    }
    exit_code
}
