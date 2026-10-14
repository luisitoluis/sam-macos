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

use eframe::egui::Color32;

/// Same theme names and colors as the terminal appearance picker.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GuiTheme {
    Aura,
    Ayu,
    Carbonfox,
    Catppuccin,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    Cobalt2,
    Cursor,
    Dracula,
    Everforest,
    Flexoki,
    Github,
    Gruvbox,
    Kanagawa,
    LucentOrng,
    Material,
    Matrix,
    Mercury,
    Monokai,
    Nightowl,
    Nord,
    OneDark,
    Opencode,
    Orng,
    OsakaJade,
    Palenight,
    Rosepine,
    Solarized,
    Synthwave84,
    Tokyonight,
    Vercel,
    Vesper,
    Zenburn,
}

pub struct Palette {
    pub text: Color32,
    pub muted: Color32,
    pub error: Color32,
    pub primary: Color32,
    pub background: Color32,
    pub border: Color32,
}

impl GuiTheme {
    pub const ALL: [GuiTheme; 33] = [
        GuiTheme::Aura,
        GuiTheme::Ayu,
        GuiTheme::Carbonfox,
        GuiTheme::Catppuccin,
        GuiTheme::CatppuccinFrappe,
        GuiTheme::CatppuccinMacchiato,
        GuiTheme::Cobalt2,
        GuiTheme::Cursor,
        GuiTheme::Dracula,
        GuiTheme::Everforest,
        GuiTheme::Flexoki,
        GuiTheme::Github,
        GuiTheme::Gruvbox,
        GuiTheme::Kanagawa,
        GuiTheme::LucentOrng,
        GuiTheme::Material,
        GuiTheme::Matrix,
        GuiTheme::Mercury,
        GuiTheme::Monokai,
        GuiTheme::Nightowl,
        GuiTheme::Nord,
        GuiTheme::OneDark,
        GuiTheme::Opencode,
        GuiTheme::Orng,
        GuiTheme::OsakaJade,
        GuiTheme::Palenight,
        GuiTheme::Rosepine,
        GuiTheme::Solarized,
        GuiTheme::Synthwave84,
        GuiTheme::Tokyonight,
        GuiTheme::Vercel,
        GuiTheme::Vesper,
        GuiTheme::Zenburn,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GuiTheme::Aura => "aura",
            GuiTheme::Ayu => "ayu",
            GuiTheme::Carbonfox => "carbonfox",
            GuiTheme::Catppuccin => "catppuccin",
            GuiTheme::CatppuccinFrappe => "catppuccin-frappe",
            GuiTheme::CatppuccinMacchiato => "catppuccin-macchiato",
            GuiTheme::Cobalt2 => "cobalt2",
            GuiTheme::Cursor => "cursor",
            GuiTheme::Dracula => "dracula",
            GuiTheme::Everforest => "everforest",
            GuiTheme::Flexoki => "flexoki",
            GuiTheme::Github => "github",
            GuiTheme::Gruvbox => "gruvbox",
            GuiTheme::Kanagawa => "kanagawa",
            GuiTheme::LucentOrng => "lucent-orng",
            GuiTheme::Material => "material",
            GuiTheme::Matrix => "matrix",
            GuiTheme::Mercury => "mercury",
            GuiTheme::Monokai => "monokai",
            GuiTheme::Nightowl => "nightowl",
            GuiTheme::Nord => "nord",
            GuiTheme::OneDark => "one-dark",
            GuiTheme::Opencode => "opencode",
            GuiTheme::Orng => "orng",
            GuiTheme::OsakaJade => "osaka-jade",
            GuiTheme::Palenight => "palenight",
            GuiTheme::Rosepine => "rosepine",
            GuiTheme::Solarized => "solarized",
            GuiTheme::Synthwave84 => "synthwave84",
            GuiTheme::Tokyonight => "tokyonight",
            GuiTheme::Vercel => "vercel",
            GuiTheme::Vesper => "vesper",
            GuiTheme::Zenburn => "zenburn",
        }
    }

    pub fn palette(self) -> Palette {
        let (text, muted, error, primary, background, border) = match self {
            GuiTheme::Aura => c(237, 236, 238, 109, 109, 109, 255, 103, 103, 162, 119, 255, 15, 15, 15, 45, 45, 45),
            GuiTheme::Ayu => c(191, 189, 182, 86, 91, 102, 217, 87, 87, 89, 194, 255, 11, 14, 20, 108, 115, 128),
            GuiTheme::Carbonfox => c(242, 244, 248, 125, 132, 143, 238, 83, 150, 51, 177, 255, 22, 22, 22, 48, 48, 48),
            GuiTheme::Catppuccin => c(205, 214, 244, 147, 153, 178, 243, 139, 168, 137, 180, 250, 30, 30, 46, 49, 50, 68),
            GuiTheme::CatppuccinFrappe => c(198, 208, 245, 148, 156, 184, 231, 130, 132, 141, 164, 226, 48, 52, 70, 65, 69, 89),
            GuiTheme::CatppuccinMacchiato => c(202, 211, 245, 147, 154, 183, 237, 135, 150, 138, 173, 244, 36, 39, 58, 54, 58, 79),
            GuiTheme::Cobalt2 => c(255, 255, 255, 173, 183, 201, 255, 0, 136, 0, 136, 255, 25, 53, 73, 31, 70, 98),
            GuiTheme::Cursor => c(228, 228, 228, 228, 228, 228, 227, 70, 113, 136, 192, 208, 24, 24, 24, 228, 228, 228),
            GuiTheme::Dracula => c(248, 248, 242, 98, 114, 164, 255, 85, 85, 189, 147, 249, 40, 42, 54, 68, 71, 90),
            GuiTheme::Everforest => c(211, 198, 170, 122, 132, 120, 230, 126, 128, 167, 192, 128, 45, 53, 59, 133, 146, 137),
            GuiTheme::Flexoki => c(206, 205, 195, 111, 110, 105, 209, 77, 65, 218, 112, 44, 16, 15, 15, 87, 86, 83),
            GuiTheme::Github => c(201, 209, 217, 139, 148, 158, 248, 81, 73, 88, 166, 255, 13, 17, 23, 48, 54, 61),
            GuiTheme::Gruvbox => c(235, 219, 178, 146, 131, 116, 251, 73, 52, 131, 165, 152, 40, 40, 40, 102, 92, 84),
            GuiTheme::Kanagawa => c(220, 215, 186, 114, 113, 105, 232, 36, 36, 126, 156, 216, 31, 31, 40, 84, 84, 109),
            GuiTheme::LucentOrng => c(238, 238, 238, 128, 128, 128, 224, 108, 117, 236, 91, 43, 42, 26, 21, 236, 91, 43),
            GuiTheme::Material => c(238, 255, 255, 84, 110, 122, 240, 113, 120, 130, 170, 255, 38, 50, 56, 55, 71, 79),
            GuiTheme::Matrix => c(98, 255, 148, 140, 163, 145, 255, 75, 75, 46, 255, 106, 10, 14, 10, 30, 42, 27),
            GuiTheme::Mercury => c(221, 221, 229, 157, 157, 168, 252, 146, 180, 141, 164, 245, 23, 23, 33, 180, 183, 200),
            GuiTheme::Monokai => c(248, 248, 242, 117, 113, 94, 249, 38, 114, 102, 217, 239, 39, 40, 34, 62, 61, 50),
            GuiTheme::Nightowl => c(214, 222, 235, 95, 126, 151, 239, 83, 80, 130, 170, 255, 1, 22, 39, 95, 126, 151),
            GuiTheme::Nord => c(236, 239, 244, 139, 149, 167, 191, 97, 106, 136, 192, 208, 46, 52, 64, 67, 76, 94),
            GuiTheme::OneDark => c(171, 178, 191, 92, 99, 112, 224, 108, 117, 97, 175, 239, 40, 44, 52, 57, 63, 74),
            GuiTheme::Opencode => c(238, 238, 238, 128, 128, 128, 224, 108, 117, 250, 178, 131, 10, 10, 10, 72, 72, 72),
            GuiTheme::Orng => c(238, 238, 238, 128, 128, 128, 224, 108, 117, 236, 91, 43, 10, 10, 10, 236, 91, 43),
            GuiTheme::OsakaJade => c(193, 196, 151, 83, 104, 91, 255, 83, 69, 45, 213, 183, 17, 28, 24, 61, 74, 68),
            GuiTheme::Palenight => c(166, 172, 205, 103, 110, 149, 240, 113, 120, 130, 170, 255, 41, 45, 62, 50, 54, 74),
            GuiTheme::Rosepine => c(224, 222, 244, 110, 106, 134, 235, 111, 146, 156, 207, 216, 25, 23, 36, 64, 61, 82),
            GuiTheme::Solarized => c(131, 148, 150, 88, 110, 117, 220, 50, 47, 38, 139, 210, 0, 43, 54, 7, 54, 66),
            GuiTheme::Synthwave84 => c(255, 255, 255, 132, 139, 189, 254, 68, 80, 54, 249, 246, 38, 35, 53, 73, 84, 149),
            GuiTheme::Tokyonight => c(200, 211, 245, 130, 139, 184, 255, 117, 127, 130, 170, 255, 26, 27, 38, 115, 122, 162),
            GuiTheme::Vercel => c(237, 237, 237, 135, 135, 135, 229, 72, 77, 0, 112, 243, 0, 0, 0, 31, 31, 31),
            GuiTheme::Vesper => c(255, 255, 255, 160, 160, 160, 255, 128, 128, 255, 199, 153, 16, 16, 16, 40, 40, 40),
            GuiTheme::Zenburn => c(220, 220, 204, 159, 159, 159, 204, 147, 147, 140, 208, 211, 63, 63, 63, 95, 95, 95),
        };
        Palette {
            text: rgb(text),
            muted: rgb(muted),
            error: rgb(error),
            primary: rgb(primary),
            background: rgb(background),
            border: rgb(border),
        }
    }
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn c(
    tr: u8, tg: u8, tb: u8, mr: u8, mg: u8, mb: u8, er: u8, eg: u8, eb: u8, pr: u8, pg: u8,
    pb: u8, br: u8, bg: u8, bb: u8, xr: u8, xg: u8, xb: u8,
) -> (
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
    (u8, u8, u8),
) {
    (
        (tr, tg, tb),
        (mr, mg, mb),
        (er, eg, eb),
        (pr, pg, pb),
        (br, bg, bb),
        (xr, xg, xb),
    )
}
