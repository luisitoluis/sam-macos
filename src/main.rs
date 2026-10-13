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

fn main() -> std::process::ExitCode {
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("The Sam window is macOS-only. Use sam-cli on this system.");
        std::process::ExitCode::FAILURE
    }

    #[cfg(target_os = "macos")]
    macos_main()
}

#[cfg(target_os = "macos")]
fn macos_main() -> std::process::ExitCode {
    use sam::backend::app::app;
    use sam::backend::orchestrator::orchestrator;
    use sam::utils::app_paths::get_executable_path;
    use sam::utils::arguments::parse_cli_arguments;
    use sam::utils::bidir_child::BidirChild;
    use std::process::Command;

    let arguments = parse_cli_arguments();

    if arguments.is_orchestrator {
        let mut tx = arguments.tx.unwrap();
        let mut rx = arguments.rx.unwrap();
        let exit_code = orchestrator(&mut tx, &mut rx);
        return std::process::ExitCode::from(exit_code);
    }

    if arguments.is_app > 0 {
        let mut tx = arguments.tx.unwrap();
        let mut rx = arguments.rx.unwrap();
        let exit_code = app(arguments.is_app, &mut tx, &mut rx);
        return std::process::ExitCode::from(exit_code);
    }

    let current_exe = get_executable_path();
    let orchestrator = BidirChild::new(Command::new(current_exe).arg("--orchestrator"))
        .expect("Failed to spawn orchestrator process");

    std::process::ExitCode::from(sam::gui_frontend::main_ui(orchestrator))
}
