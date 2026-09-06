use super::app::App;
use super::video_worker::{VideoFrame, VideoWorker};
use macroquad::prelude::*;
use renrs::WaitState;
use renrs::runtime::VisualEffect;

pub(super) struct StreamingVideo {
    path: String,
    worker: VideoWorker,
    texture: Option<Texture2D>,
    pending: Option<VideoFrame>,
    current: Option<usize>,
    requested: usize,
    failed: bool,
}

impl App {
    pub(super) fn sync_video_audio(&mut self) {
        let state = self
            .runtime
            .as_ref()
            .and_then(renrs::Runtime::waiting)
            .and_then(|wait| {
                if let WaitState::Effect {
                    effect: VisualEffect::Video { path, seconds },
                } = wait
                {
                    self.clips
                        .get(path)
                        .and_then(|clip| clip.audio.clone())
                        .map(|path| (path, seconds - self.effect_remaining))
                } else {
                    None
                }
            });
        let paused = self.overlay.is_some()
            || !self.media_ready()
            || !self.assets.is_ready()
            || self.storage.loading
            || self.storage.quit_after_save;
        if let Some((path, position)) = state {
            self.audio.sync_video(Some(path), position, paused);
        } else {
            self.audio.stop_video();
        }
    }
    pub(super) fn video_audio_position(&self) -> Option<f32> {
        if let Some(WaitState::Effect {
            effect: VisualEffect::Video { path, .. },
        }) = self.runtime.as_ref().and_then(renrs::Runtime::waiting)
            && self
                .clips
                .get(path)
                .is_some_and(|clip| clip.audio.is_some())
        {
            return self.audio.video_position();
        }
        None
    }
    pub(super) fn media_paths(&mut self) -> Vec<String> {
        let Some(WaitState::Effect { effect }) =
            self.runtime.as_ref().and_then(renrs::Runtime::waiting)
        else {
            self.video = None;
            return Vec::new();
        };
        match effect {
            VisualEffect::Dissolve { from, .. } => from
                .background
                .iter()
                .cloned()
                .chain(
                    from.sprites
                        .iter()
                        .flat_map(renrs::runtime::SpriteState::image_paths),
                )
                .collect(),
            VisualEffect::Video { path, seconds } => {
                if !self.clips.contains_key(path) {
                    match self
                        .source
                        .read(path)
                        .map_err(|error| error.to_string())
                        .and_then(|bytes| renrs::video::VideoClip::from_slice(&bytes))
                    {
                        Ok(clip) => {
                            self.clips.insert(path.clone(), clip);
                        }
                        Err(error) => {
                            self.fatal_error = Some(format!("Video {path}: {error}"));
                            return Vec::new();
                        }
                    }
                }
                let clip = &self.clips[path];
                if let Some(stream) = &clip.stream {
                    let requested = clip.frame(seconds - self.effect_remaining);
                    if self.video.as_ref().is_none_or(|video| &video.path != path) {
                        self.video = Some(StreamingVideo {
                            path: path.clone(),
                            worker: VideoWorker::new(self.source.clone(), clip.clone(), requested),
                            texture: None,
                            pending: None,
                            current: None,
                            requested,
                            failed: false,
                        });
                    }
                    let video = self.video.as_mut().unwrap();
                    video.requested = requested;
                    for _ in 0..4 {
                        if video.pending.is_none() {
                            match video.worker.poll() {
                                Some(Ok(frame)) => video.pending = Some(frame),
                                Some(Err(error)) => {
                                    video.failed = true;
                                    self.fatal_error = Some(error);
                                    break;
                                }
                                None => break,
                            }
                        }
                        if video
                            .pending
                            .as_ref()
                            .is_some_and(|frame| frame.index > requested)
                        {
                            break;
                        }
                        if let Some(frame) = video.pending.take() {
                            let image = Image {
                                width: stream.width,
                                height: stream.height,
                                bytes: frame.rgba,
                            };
                            if let Some(texture) = &video.texture {
                                texture.update(&image);
                            } else {
                                video.texture = Some(Texture2D::from_image(&image));
                            }
                            video.current = Some(frame.index);
                        }
                    }
                    return Vec::new();
                }
                self.video = None;
                clip.frames
                    .iter()
                    .skip(clip.frame(seconds - self.effect_remaining))
                    .take(3)
                    .cloned()
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn draw_media(&self) {
        let Some(WaitState::Effect { effect }) =
            self.runtime.as_ref().and_then(renrs::Runtime::waiting)
        else {
            return;
        };
        match effect {
            VisualEffect::Video { path, seconds } => {
                if let Some(texture) = self.video.as_ref().and_then(|video| video.texture.as_ref())
                {
                    let scale = (super::CANVAS_WIDTH / texture.width())
                        .min(super::CANVAS_HEIGHT / texture.height());
                    clear_background(BLACK);
                    draw_texture_ex(
                        texture,
                        (super::CANVAS_WIDTH - texture.width() * scale) / 2.0,
                        (super::CANVAS_HEIGHT - texture.height() * scale) / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(
                                texture.width() * scale,
                                texture.height() * scale,
                            )),
                            ..Default::default()
                        },
                    );
                    return;
                }
                if let Some(clip) = self.clips.get(path) {
                    self.draw_background(
                        clip.frames
                            .get(clip.frame(seconds - self.effect_remaining))
                            .map(String::as_str),
                    );
                }
            }
            VisualEffect::Dissolve { from, seconds } if !self.theme.reduced_motion => {
                let alpha = if *seconds > f32::EPSILON {
                    (self.effect_remaining / seconds).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                self.draw_stage_tinted(from, alpha);
            }
            _ => {}
        }
    }

    pub(super) fn media_ready(&self) -> bool {
        self.video.as_ref().is_none_or(|video| {
            video.failed || video.current.is_some_and(|index| index >= video.requested)
        })
    }
}
