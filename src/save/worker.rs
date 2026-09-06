use super::{SaveFile, SaveMetadata, SaveRepository, SaveSlot};
use crate::runtime::RuntimeSnapshot;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

pub struct SaveThumbnail {
    /// Bottom-up 240x135 RGBA pixels from the save-time render pass.
    pub rgba: Vec<u8>,
}

impl SaveThumbnail {
    fn encode(self) -> Result<Vec<u8>, String> {
        let mut image = image::RgbaImage::from_raw(240, 135, self.rgba)
            .ok_or("invalid save thumbnail dimensions")?;
        image::imageops::flip_vertical_in_place(&mut image);
        let mut output = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut output, image::ImageOutputFormat::Png)
            .map_err(|error| error.to_string())?;
        Ok(output.into_inner())
    }
}

pub enum SaveRequest {
    Save {
        slot: String,
        rotation: Option<usize>,
        snapshot: Box<RuntimeSnapshot>,
        metadata: SaveMetadata,
        thumbnail: Option<SaveThumbnail>,
    },
    Load(String),
    List,
    Delete(String),
    Import {
        path: PathBuf,
        slot: String,
        project_id: String,
    },
    Export {
        path: PathBuf,
        slot: String,
    },
    Persist {
        path: PathBuf,
        bytes: Vec<u8>,
    },
}

pub enum SaveResponse {
    Saved(String),
    Loaded(String, Box<SaveFile>),
    Listed(Vec<SaveSlot>),
    Completed(String),
    Error(String),
}

/// A bounded, ordered storage queue. Dropping it drains accepted writes before joining.
pub struct SaveWorker {
    requests: Option<SyncSender<SaveRequest>>,
    responses: Receiver<SaveResponse>,
    thread: Option<JoinHandle<()>>,
    pending: usize,
}

impl SaveWorker {
    #[must_use]
    pub fn new(repository: SaveRepository) -> Self {
        let (requests, incoming) = mpsc::sync_channel(8);
        let (outgoing, responses) = mpsc::channel();
        let thread = thread::spawn(move || {
            while let Ok(request) = incoming.recv() {
                let response = execute(&repository, request).unwrap_or_else(SaveResponse::Error);
                if outgoing.send(response).is_err() {
                    break;
                }
            }
        });
        Self {
            requests: Some(requests),
            responses,
            thread: Some(thread),
            pending: 0,
        }
    }

    /// Queues a storage command without blocking the caller.
    ///
    /// # Errors
    /// Returns an error if the bounded queue is full or the worker stopped.
    pub fn submit(&mut self, request: SaveRequest) -> Result<(), String> {
        self.requests
            .as_ref()
            .ok_or("storage worker stopped")?
            .try_send(request)
            .map_err(|error| format!("storage queue unavailable: {error}"))?;
        self.pending += 1;
        Ok(())
    }

    pub fn poll(&mut self) -> Option<SaveResponse> {
        let response = self.responses.try_recv().ok()?;
        self.pending = self.pending.saturating_sub(1);
        Some(response)
    }

    #[must_use]
    pub const fn busy(&self) -> bool {
        self.pending > 0
    }
}

impl Drop for SaveWorker {
    fn drop(&mut self) {
        self.requests.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn execute(repository: &SaveRepository, request: SaveRequest) -> Result<SaveResponse, String> {
    let result = match request {
        SaveRequest::Save {
            slot,
            rotation,
            snapshot,
            mut metadata,
            thumbnail,
        } => {
            if let Some(thumbnail) = thumbnail {
                metadata
                    .presentation
                    .get_or_insert_with(Default::default)
                    .thumbnail_png = thumbnail.encode()?;
            }
            if let Some(count) = rotation {
                repository.save_rotating_owned(&slot, count, *snapshot, metadata)
            } else {
                repository.save_owned(&slot, *snapshot, metadata)
            }
            .map(|()| SaveResponse::Saved(slot))
        }
        SaveRequest::Load(slot) => repository
            .load(&slot)
            .map(|save| SaveResponse::Loaded(slot, Box::new(save))),
        SaveRequest::List => repository.list_cached().map(SaveResponse::Listed),
        SaveRequest::Delete(slot) => repository
            .delete(&slot)
            .map(|()| SaveResponse::Completed("Deleted".to_owned())),
        SaveRequest::Export { path, slot } => repository
            .export(&slot, &path)
            .map(|()| SaveResponse::Completed("Exported".to_owned())),
        SaveRequest::Import {
            path,
            slot,
            project_id,
        } => {
            let save: SaveFile = serde_json::from_reader(std::io::BufReader::new(
                std::fs::File::open(&path).map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string())?;
            if !save.project_id.is_empty() && save.project_id != project_id {
                return Err("Save belongs to another game".to_owned());
            }
            repository
                .import(&path, &slot)
                .map(|()| SaveResponse::Completed("Imported".to_owned()))
        }
        SaveRequest::Persist { path, bytes } => {
            return crate::storage::atomic_write(&path, |file| {
                use std::io::Write;
                file.write_all(&bytes)
            })
            .map(|()| SaveResponse::Completed(String::new()))
            .map_err(|e| e.to_string());
        }
    };
    result.map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_writes_finish_before_worker_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let repo = SaveRepository::new(root.path());
        let runtime = crate::Runtime::new(
            crate::compile(
                &crate::parse_script("label start:\n    \"Hello\"", "test.rns").unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let mut worker = SaveWorker::new(repo.clone());
        worker
            .submit(SaveRequest::Save {
                slot: "slot-1".to_owned(),
                rotation: None,
                snapshot: Box::new(runtime.snapshot()),
                metadata: SaveMetadata::default(),
                thumbnail: Some(SaveThumbnail {
                    rgba: vec![127; 240 * 135 * 4],
                }),
            })
            .unwrap();
        drop(worker);
        let saved = repo.load("slot-1").unwrap();
        let image = image::load_from_memory(&saved.presentation.unwrap().thumbnail_png).unwrap();
        assert_eq!((image.width(), image.height()), (240, 135));
    }
}
