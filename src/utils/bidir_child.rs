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

use crate::utils::ipc_types::SamError;
use interprocess::unnamed_pipe::pipe;
use interprocess::unnamed_pipe::{Recver, Sender};
use std::os::fd::IntoRawFd;
use std::process::{Child, Command};

#[derive(Debug)]
pub struct BidirChild {
    pub child: Child,
    pub tx: Sender,
    pub rx: Recver,
}

impl BidirChild {
    pub fn new(command: &mut Command) -> Result<Self, SamError> {
        let (parent_to_child_tx, parent_to_child_rx) = pipe().expect("Unable to create a pipe");
        let (child_to_parent_tx, child_to_parent_rx) = pipe().expect("Unable to create a pipe");

        let child_to_parent_tx_handle: i32 = child_to_parent_tx.into_raw_fd();
        let parent_to_child_rx_handle: i32 = parent_to_child_rx.into_raw_fd();

        let child = match command
            .arg(format!("--tx={child_to_parent_tx_handle}"))
            .arg(format!("--rx={parent_to_child_rx_handle}"))
            .spawn()
        {
            Ok(child) => {
                // We don't need to close the ends we don't need, they are already consumed
                // drop(parent_to_child_rx);
                // drop(child_to_parent_tx);

                child
            }
            Err(_) => {
                eprintln!("Unable to spawn a child process");
                return Err(SamError::UnknownError);
            }
        };

        Ok(Self {
            child,
            tx: parent_to_child_tx,
            rx: child_to_parent_rx,
        })
    }
}
