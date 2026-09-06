use renrs::{ProjectSource, video::VideoClip};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender},
};

pub(super) struct VideoFrame {
    pub(super) index: usize,
    pub(super) rgba: Vec<u8>,
}
pub(super) struct VideoWorker {
    frames: Receiver<Result<VideoFrame, String>>,
    child: Arc<Mutex<Option<Child>>>,
    stopped: Arc<std::sync::atomic::AtomicBool>,
}

impl VideoWorker {
    pub(super) fn new(source: ProjectSource, clip: VideoClip, first: usize) -> Self {
        let (sender, frames) = mpsc::sync_channel(2);
        let child = Arc::new(Mutex::new(None));
        let process = child.clone();
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled = stopped.clone();
        std::thread::spawn(move || {
            if let Err(error) = decode(&source, &clip, first, &sender, &process, &cancelled) {
                let _ = sender.send(Err(error));
            }
            if let Some(mut child) = process.lock().unwrap().take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        });
        Self {
            frames,
            child,
            stopped,
        }
    }
    pub(super) fn poll(&self) -> Option<Result<VideoFrame, String>> {
        self.frames.try_recv().ok()
    }
}

impl Drop for VideoWorker {
    fn drop(&mut self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
}

fn decode(
    source: &ProjectSource,
    clip: &VideoClip,
    first: usize,
    sender: &SyncSender<Result<VideoFrame, String>>,
    process: &Mutex<Option<Child>>,
    stopped: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let stream = clip.stream.as_ref().ok_or("missing video stream")?;
    let extracted;
    let input = match source {
        ProjectSource::Directory(root) => renrs::resources::resource_path(root, &stream.path)
            .map_err(|error| error.to_string())?,
        ProjectSource::Archive(_) => {
            extracted = tempfile::NamedTempFile::new().map_err(|error| error.to_string())?;
            std::io::copy(
                &mut source
                    .open_reader(&stream.path)
                    .map_err(|error| error.to_string())?,
                &mut extracted.as_file(),
            )
            .map_err(|error| error.to_string())?;
            extracted.path().to_owned()
        }
    };
    let mut child =
        Command::new(std::env::var_os("RENRS_FFMPEG").unwrap_or_else(|| "ffmpeg".into()))
            .args([
                "-nostdin",
                "-v",
                "error",
                "-threads",
                "1",
                "-ss",
                &(first as f32 / clip.fps).to_string(),
                "-i",
            ])
            .arg(input)
            .args([
                "-an",
                "-vf",
                &format!("fps={},scale={}:{}", clip.fps, stream.width, stream.height),
                "-threads",
                "1",
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("streamed video requires ffmpeg or RENRS_FFMPEG: {error}"))?;
    let mut output = child.stdout.take().ok_or("video decoder has no output")?;
    {
        let mut active = process.lock().unwrap();
        if stopped.load(std::sync::atomic::Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(());
        }
        *active = Some(child);
    }
    for index in first..(clip.duration() * clip.fps).ceil() as usize {
        let mut rgba = vec![0; usize::from(stream.width) * usize::from(stream.height) * 4];
        output
            .read_exact(&mut rgba)
            .map_err(|error| format!("video decode ended before frame {index}: {error}"))?;
        if sender.send(Ok(VideoFrame { index, rgba })).is_err() {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires ffmpeg or RENRS_FFMPEG"]
    fn streams_frames_from_a_saved_offset() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("video.mp4");
        let status =
            Command::new(std::env::var_os("RENRS_FFMPEG").unwrap_or_else(|| "ffmpeg".into()))
                .args([
                    "-nostdin",
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=320x180:rate=24",
                    "-t",
                    "1",
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                ])
                .arg(&path)
                .status()
                .unwrap();
        assert!(status.success());
        let clip = VideoClip {
            version: 2,
            fps: 24.0,
            frames: Vec::new(),
            audio: None,
            stream: Some(renrs::video::VideoStream {
                path: "video.mp4".to_owned(),
                seconds: 1.0,
                width: 320,
                height: 180,
            }),
        };
        let worker = VideoWorker::new(ProjectSource::Directory(root.path().to_owned()), clip, 12);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut frames = Vec::new();
        while frames.len() < 12 {
            assert!(
                std::time::Instant::now() < deadline,
                "video worker timed out"
            );
            if let Some(frame) = worker.poll() {
                frames.push(frame.unwrap());
            } else {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        assert_eq!(frames[0].index, 12);
        assert_eq!(frames[11].index, 23);
        assert_eq!(frames[0].rgba.len(), 320 * 180 * 4);
        assert_ne!(frames[0].rgba, frames[11].rgba);
    }
}
