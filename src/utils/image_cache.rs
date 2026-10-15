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

use crate::utils::app_paths::get_app_cache_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

const WORKERS: usize = 6;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

type Done = Box<dyn FnOnce(Option<Vec<u8>>) + Send>;

struct Job {
    url: String,
    done: Done,
}

#[derive(Default)]
struct Queue {
    jobs: Vec<Job>,
    closed: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
}

/// Downloads images through a disk cache shared by the GUI and the TUI.
///
/// Requests are served newest first, so whatever the user is looking at right
/// now is fetched before rows they already scrolled past.
pub struct ImageFetcher {
    shared: Arc<Shared>,
}

impl ImageFetcher {
    pub fn new() -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
        });
        let dir = cache_dir();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_max_idle_per_host(WORKERS)
            .build()
            .ok();
        for _ in 0..WORKERS {
            let shared = Arc::clone(&shared);
            let dir = dir.clone();
            let client = client.clone();
            thread::spawn(move || worker(&shared, dir, client));
        }
        Self { shared }
    }

    /// Calls `done` on a worker thread with the image bytes, or `None` if they
    /// could not be read from disk or downloaded.
    pub fn fetch(&self, url: &str, done: impl FnOnce(Option<Vec<u8>>) + Send + 'static) {
        let Ok(mut queue) = self.shared.queue.lock() else {
            return;
        };
        queue.jobs.push(Job {
            url: url.to_owned(),
            done: Box::new(done),
        });
        drop(queue);
        self.shared.wake.notify_one();
    }
}

impl Default for ImageFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ImageFetcher {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.shared.queue.lock() {
            queue.closed = true;
            queue.jobs.clear();
        }
        self.shared.wake.notify_all();
    }
}

fn worker(shared: &Shared, dir: Option<PathBuf>, client: Option<reqwest::blocking::Client>) {
    loop {
        let job = {
            let Ok(mut queue) = shared.queue.lock() else {
                return;
            };
            loop {
                if queue.closed {
                    return;
                }
                if let Some(job) = queue.jobs.pop() {
                    break job;
                }
                queue = match shared.wake.wait(queue) {
                    Ok(queue) => queue,
                    Err(_) => return,
                };
            }
        };
        let path = dir.as_ref().map(|dir| dir.join(file_name(&job.url)));
        let bytes = path
            .as_deref()
            .and_then(read_fresh)
            .or_else(|| {
                let bytes = download(client.as_ref()?, &job.url)?;
                if let Some(path) = &path {
                    store(path, &bytes);
                }
                Some(bytes)
            });
        (job.done)(bytes);
    }
}

fn cache_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(get_app_cache_dir()).join("images");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// FNV-1a, so names stay the same across Rust versions.
fn file_name(url: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in url.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn read_fresh(path: &Path) -> Option<Vec<u8>> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).unwrap_or_default();
    if age > MAX_AGE {
        return None;
    }
    fs::read(path).ok().filter(|bytes| !bytes.is_empty())
}

fn download(client: &reqwest::blocking::Client, url: &str) -> Option<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.bytes())
        .ok()?;
    (!bytes.is_empty()).then(|| bytes.to_vec())
}

/// Writes through a temporary file so a reader never sees half an image.
fn store(path: &Path, bytes: &[u8]) {
    let temp = path.with_extension(format!("tmp{:?}", thread::current().id()));
    if fs::write(&temp, bytes).is_ok() && fs::rename(&temp, path).is_err() {
        let _ = fs::remove_file(&temp);
    }
}
