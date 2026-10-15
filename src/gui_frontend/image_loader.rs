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
use eframe::egui;
use egui::load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
enum Entry {
    Pending,
    Ready(Arc<[u8]>),
    Failed,
}

/// Serves `http(s)` image URIs from the shared disk cache, downloading on a
/// fixed worker pool instead of one thread per image.
pub struct CachedHttpLoader {
    fetcher: ImageFetcher,
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl CachedHttpLoader {
    pub const ID: &'static str = egui::generate_loader_id!(CachedHttpLoader);

    pub fn new() -> Self {
        Self {
            fetcher: ImageFetcher::new(),
            cache: Arc::default(),
        }
    }
}

impl BytesLoader for CachedHttpLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        if !(uri.starts_with("https://") || uri.starts_with("http://")) {
            return Err(LoadError::NotSupported);
        }
        let Ok(mut cache) = self.cache.lock() else {
            return Err(LoadError::Loading("image cache is poisoned".to_owned()));
        };
        match cache.get(uri) {
            Some(Entry::Ready(bytes)) => {
                return Ok(BytesPoll::Ready {
                    size: None,
                    bytes: Bytes::Shared(Arc::clone(bytes)),
                    mime: None,
                });
            }
            Some(Entry::Pending) => return Ok(BytesPoll::Pending { size: None }),
            Some(Entry::Failed) => return Err(LoadError::Loading(format!("could not load {uri}"))),
            None => {}
        }
        cache.insert(uri.to_owned(), Entry::Pending);
        drop(cache);

        let key = uri.to_owned();
        let cache = Arc::clone(&self.cache);
        let ctx = ctx.clone();
        self.fetcher.fetch(uri, move |bytes| {
            let entry = match bytes {
                Some(bytes) => Entry::Ready(bytes.into()),
                None => Entry::Failed,
            };
            let still_wanted = cache
                .lock()
                .map(|mut cache| match cache.get_mut(&key) {
                    Some(slot) => {
                        *slot = entry;
                        true
                    }
                    None => false,
                })
                .unwrap_or(false);
            // The cache lock must be released before touching the context.
            if still_wanted {
                ctx.request_repaint();
            }
        });
        Ok(BytesPoll::Pending { size: None })
    }

    fn forget(&self, uri: &str) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(uri);
        }
    }

    fn forget_all(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .map(|cache| {
                cache
                    .values()
                    .map(|entry| match entry {
                        Entry::Ready(bytes) => bytes.len(),
                        _ => 0,
                    })
                    .sum()
            })
            .unwrap_or(0)
    }

    fn has_pending(&self) -> bool {
        self.cache
            .lock()
            .map(|cache| cache.values().any(|entry| matches!(entry, Entry::Pending)))
            .unwrap_or(false)
    }
}
