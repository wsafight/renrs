use super::byte_budget::{ByteBudget, Reservation};
use renrs::ProjectSource;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

const ENCODED_LIMIT: usize = 32 * 1024 * 1024;
const DECODE_LIMIT: usize = 256 * 1024 * 1024;

pub(super) struct DecodedImage {
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) rgba: Vec<u8>,
    _reservation: Reservation,
}

pub(super) struct AssetWorker {
    requests: SyncSender<(String, u64)>,
    responses: Receiver<(String, u64, Result<DecodedImage, String>)>,
    pending: HashSet<(String, u64)>,
    budget: Arc<ByteBudget>,
}

impl AssetWorker {
    pub(super) fn new(source: ProjectSource) -> Self {
        let (requests, receiver) = sync_channel::<(String, u64)>(16);
        let (sender, responses) = sync_channel(8);
        let budget = ByteBudget::new(DECODE_LIMIT);
        let decoding_budget = budget.clone();
        thread::spawn(move || {
            while let Ok((path, generation)) = receiver.recv() {
                let decoded = decode(&source, &path, &decoding_budget);
                if sender.send((path, generation, decoded)).is_err() {
                    break;
                }
            }
        });
        Self {
            requests,
            responses,
            pending: HashSet::new(),
            budget,
        }
    }

    pub(super) fn request(&mut self, path: &str, generation: u64) {
        let key = (path.to_owned(), generation);
        if !self.pending.contains(&key)
            && self
                .requests
                .try_send((path.to_owned(), generation))
                .is_ok()
        {
            self.pending.insert(key);
        }
    }

    pub(super) fn poll(&mut self) -> Option<(String, u64, Result<DecodedImage, String>)> {
        let response = self.responses.try_recv().ok()?;
        self.pending.remove(&(response.0.clone(), response.1));
        Some(response)
    }

    pub(super) fn queued_bytes(&self) -> usize {
        self.budget.used()
    }
}

impl Drop for AssetWorker {
    fn drop(&mut self) {
        self.budget.close();
    }
}

fn decode(
    source: &ProjectSource,
    path: &str,
    budget: &Arc<ByteBudget>,
) -> Result<DecodedImage, String> {
    let _encoded = budget.reserve(ENCODED_LIMIT)?;
    let bytes = source
        .read_limited(path, ENCODED_LIMIT)
        .map_err(|error| error.to_string())?;
    let dimensions = image::io::Reader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?
        .into_dimensions()
        .map_err(|error| error.to_string())?;
    // Reserve decoder scratch, source pixels and RGBA conversion before allocation.
    let required = u64::from(dimensions.0) * u64::from(dimensions.1) * 12 + 1024 * 1024;
    let required = usize::try_from(required).map_err(|error| error.to_string())?;
    if required > DECODE_LIMIT - ENCODED_LIMIT {
        return Err("image exceeds decode byte budget".to_owned());
    }
    let reservation = budget.reserve(required)?;
    let mut reader = image::io::Reader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(required as u64 / 3);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|error| error.to_string())?
        .into_rgba8();
    Ok(DecodedImage {
        width: u16::try_from(image.width()).map_err(|error| error.to_string())?,
        height: u16::try_from(image.height()).map_err(|error| error.to_string())?,
        rgba: image.into_raw(),
        _reservation: reservation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_reports_bad_images_without_panicking_or_a_gpu() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("bad.png"), b"not an image").unwrap();
        assert!(
            decode(
                &ProjectSource::Directory(root.path().to_owned()),
                "bad.png",
                &ByteBudget::new(DECODE_LIMIT)
            )
            .is_err()
        );
    }
}
