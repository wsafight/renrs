use super::asset_worker::AssetWorker;
use macroquad::prelude::*;
use renrs::ProjectSource;
use std::collections::{HashMap, HashSet};

pub(super) struct AssetCache {
    pub(super) textures: HashMap<String, Texture2D>,
    failed: HashSet<String>,
    usage: HashMap<String, (u64, usize)>,
    worker: Option<AssetWorker>,
    generation: u64,
    frame: u64,
    budget: usize,
    notices: Vec<String>,
    ready: bool,
    resident: usize,
    revision: u64,
}

impl Default for AssetCache {
    fn default() -> Self {
        Self {
            textures: HashMap::new(),
            failed: HashSet::new(),
            usage: HashMap::new(),
            worker: None,
            generation: 0,
            frame: 0,
            budget: 256 * 1024 * 1024,
            notices: Vec::new(),
            ready: false,
            resident: 0,
            revision: 0,
        }
    }
}

impl AssetCache {
    pub(super) fn prepare(&mut self, source: &ProjectSource, paths: &[String], hints: &[String]) {
        self.frame += 1;
        let prefetch = self.resident_bytes() < self.budget;
        self.worker
            .get_or_insert_with(|| AssetWorker::new(source.clone()));
        while let Some((path, generation, decoded)) = self.worker.as_mut().unwrap().poll() {
            if generation != self.generation {
                continue;
            }
            self.revision = self.revision.wrapping_add(1);
            match decoded {
                Ok(image) if image.rgba.len() <= self.budget => {
                    self.trim_to(paths, self.budget - image.rgba.len());
                    if self.resident + image.rgba.len() > self.budget {
                        self.failed.insert(path.clone());
                        self.notices
                            .push(format!("{path}: visible images exceed texture budget"));
                    } else {
                        let texture = Texture2D::from_rgba8(image.width, image.height, &image.rgba);
                        texture.set_filter(FilterMode::Linear);
                        self.usage
                            .insert(path.clone(), (self.frame, image.rgba.len()));
                        self.textures.insert(path, texture);
                        self.resident += image.rgba.len();
                    }
                }
                result => {
                    let reason = result
                        .err()
                        .unwrap_or_else(|| "image exceeds texture budget".to_owned());
                    self.notices.push(format!("{path}: {reason}"));
                    self.failed.insert(path);
                }
            }
            break;
        }
        for path in paths.iter().chain(hints.iter().filter(|_| prefetch)) {
            if let Some((last_used, _)) = self.usage.get_mut(path) {
                *last_used = self.frame;
            }
            if !self.textures.contains_key(path) && !self.failed.contains(path) {
                self.worker.as_mut().unwrap().request(path, self.generation);
            }
        }
        self.ready = paths
            .iter()
            .all(|path| self.textures.contains_key(path) || self.failed.contains(path));
    }

    fn trim_to(&mut self, pinned: &[String], target: usize) {
        if self.resident <= target {
            return;
        }
        let mut candidates: Vec<_> = self
            .usage
            .iter()
            .filter(|(path, _)| !pinned.contains(path))
            .map(|(path, (time, size))| (path.clone(), *time, *size))
            .collect();
        candidates.sort_by_key(|(_, time, _)| *time);
        for (path, _, size) in candidates {
            if self.resident <= target {
                break;
            }
            self.textures.remove(&path);
            self.usage.remove(&path);
            self.resident -= size;
        }
    }

    pub(super) fn invalidate(&mut self, paths: &[String]) {
        self.revision = self.revision.wrapping_add(1);
        self.generation += 1;
        for path in paths {
            self.textures.remove(path);
            if let Some((_, size)) = self.usage.remove(path) {
                self.resident -= size;
            }
            self.failed.remove(path);
        }
    }

    pub(super) fn take_notice(&mut self) -> Option<String> {
        self.notices.pop()
    }

    pub(super) const fn is_ready(&self) -> bool {
        self.ready
    }

    pub(super) fn resident_bytes(&self) -> usize {
        self.resident
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) fn queued_bytes(&self) -> usize {
        self.worker.as_ref().map_or(0, AssetWorker::queued_bytes)
    }
}
