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

use crate::utils::image_cache::ImageFetcher;
use image::DynamicImage;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{FilterType, Resize, StatefulImage};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};

/// Decoded images kept in memory. Past this, finished entries are dropped and
/// read again from the disk cache when shown.
const MAX_DECODED: usize = 256;
/// Encoded protocols are tied to one render size, so each place an image is
/// shown gets its own entry.
const MAX_PROTOCOLS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    LibraryPreview,
    GameHeader,
    AchievementIcon,
}

enum Decoded {
    Loading,
    Ready(DynamicImage),
    Missing,
}

pub struct Images {
    picker: Picker,
    decoded: HashMap<String, Decoded>,
    protocols: HashMap<(String, Slot), StatefulProtocol>,
    fetcher: ImageFetcher,
    loaded_tx: Sender<(String, Option<DynamicImage>)>,
    loaded: Receiver<(String, Option<DynamicImage>)>,
    shown_before: HashMap<Slot, (String, Rect)>,
    shown_now: HashMap<Slot, (String, Rect)>,
}

impl Images {
    /// Must run after raw mode and the alternate screen are on, and before the
    /// first terminal event is read: the picker queries the terminal on stdio.
    pub fn new() -> Self {
        let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        let (loaded_tx, loaded_rx) = mpsc::channel();
        Self {
            picker,
            decoded: HashMap::new(),
            protocols: HashMap::new(),
            fetcher: ImageFetcher::new(),
            loaded_tx,
            loaded: loaded_rx,
            shown_before: HashMap::new(),
            shown_now: HashMap::new(),
        }
    }

    /// Call once at the start of every frame, before any `render`.
    pub fn begin_frame(&mut self) {
        self.shown_before = std::mem::take(&mut self.shown_now);
    }

    pub fn poll(&mut self) {
        while let Ok((url, image)) = self.loaded.try_recv() {
            let entry = match image {
                Some(image) => Decoded::Ready(image),
                None => Decoded::Missing,
            };
            self.decoded.insert(url, entry);
        }
    }

    /// Draws the image at `url` into `area`, or nothing while it is still
    /// downloading or if it could not be loaded.
    ///
    /// A slot whose image changes is left empty for one frame. Pixel protocols
    /// only rewrite the first cell of the area, and some terminals (Warp) keep
    /// the old picture unless its cells are overwritten with plain text first.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, slot: Slot, url: &str) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if self
            .shown_before
            .get(&slot)
            .is_some_and(|(shown_url, shown_area)| shown_url != url || *shown_area != area)
        {
            self.prefetch(url);
            return;
        }
        let key = (url.to_owned(), slot);
        if !self.protocols.contains_key(&key) {
            let Some(image) = self.image(url) else {
                return;
            };
            if self.protocols.len() >= MAX_PROTOCOLS {
                self.protocols.clear();
            }
            let protocol = self.picker.new_resize_protocol(image);
            self.protocols.insert(key.clone(), protocol);
        }
        if let Some(protocol) = self.protocols.get_mut(&key) {
            frame.render_stateful_widget(
                StatefulImage::default().resize(Resize::Fit(Some(FilterType::Triangle))),
                area,
                protocol,
            );
            self.shown_now.insert(slot, (url.to_owned(), area));
        }
    }

    fn image(&mut self, url: &str) -> Option<DynamicImage> {
        match self.decoded.get(url) {
            Some(Decoded::Ready(image)) => return Some(image.clone()),
            Some(Decoded::Loading | Decoded::Missing) => return None,
            None => {}
        }
        if self.decoded.len() >= MAX_DECODED {
            self.decoded
                .retain(|_, entry| matches!(entry, Decoded::Loading));
        }
        self.decoded.insert(url.to_owned(), Decoded::Loading);
        let loaded = self.loaded_tx.clone();
        let key = url.to_owned();
        self.fetcher.fetch(url, move |bytes| {
            let image = bytes.and_then(|bytes| image::load_from_memory(&bytes).ok());
            let _ = loaded.send((key, image));
        });
        None
    }

    /// Starts loading `url` in the background so it is ready when shown.
    pub fn prefetch(&mut self, url: &str) {
        if !self.decoded.contains_key(url) {
            let _ = self.image(url);
        }
    }
}
