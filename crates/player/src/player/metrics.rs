use serde::Serialize;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

#[derive(Serialize)]
pub(super) struct FrameReport {
    frames: usize,
    scene_redraws: usize,
    frame_p50_ms: f64,
    frame_p95_ms: f64,
    frame_p99_ms: f64,
    frame_max_ms: f64,
    peak_rss_bytes: u64,
    peak_texture_bytes: usize,
    peak_image_decode_reserved_bytes: usize,
}

pub(super) struct Metrics {
    samples: Vec<f64>,
    maximum: f64,
    frames: usize,
    scene_redraws: usize,
    memory: Receiver<u64>,
    stop: Sender<()>,
    peak_rss: u64,
    peak_texture: usize,
    peak_image_decode: usize,
}

impl Default for Metrics {
    fn default() -> Self {
        let (send, memory) = mpsc::channel();
        let (stop, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut system = sysinfo::System::new();
            let pid = sysinfo::Pid::from_u32(std::process::id());
            loop {
                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    sysinfo::ProcessRefreshKind::new().with_memory(),
                );
                if let Some(process) = system.process(pid)
                    && send.send(process.memory()).is_err()
                {
                    break;
                }
                match receive.recv_timeout(Duration::from_secs(1)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    _ => break,
                }
            }
        });
        Self {
            samples: Vec::new(),
            maximum: 0.0,
            frames: 0,
            scene_redraws: 0,
            memory,
            stop,
            peak_rss: 0,
            peak_texture: 0,
            peak_image_decode: 0,
        }
    }
}

impl Metrics {
    pub(super) const fn redraw_count(&self) -> usize {
        self.scene_redraws
    }
    pub(super) fn redraw(&mut self) {
        self.scene_redraws += 1;
    }
    pub(super) fn record(&mut self, seconds: f32, texture_bytes: usize, decode_bytes: usize) {
        let ms = f64::from(seconds) * 1000.0;
        if ms.is_finite() && ms > 0.0 {
            self.maximum = self.maximum.max(ms);
            self.frames += 1;
            if self.samples.len() < 30_000 {
                self.samples.push(ms);
            } else {
                let index = self.frames % 30_000;
                self.samples[index] = ms;
            }
        }
        for rss in self.memory.try_iter() {
            self.peak_rss = self.peak_rss.max(rss);
        }
        self.peak_texture = self.peak_texture.max(texture_bytes);
        self.peak_image_decode = self.peak_image_decode.max(decode_bytes);
    }
    pub(super) fn report(&self) -> FrameReport {
        let mut samples = self.samples.clone();
        samples.sort_by(f64::total_cmp);
        let percentile = |value: f64| {
            samples
                .get(((samples.len().saturating_sub(1) as f64) * value).ceil() as usize)
                .copied()
                .unwrap_or(0.0)
        };
        FrameReport {
            frames: self.frames,
            scene_redraws: self.scene_redraws,
            frame_p50_ms: percentile(0.5),
            frame_p95_ms: percentile(0.95),
            frame_p99_ms: percentile(0.99),
            frame_max_ms: self.maximum,
            peak_rss_bytes: self.peak_rss,
            peak_texture_bytes: self.peak_texture,
            peak_image_decode_reserved_bytes: self.peak_image_decode,
        }
    }
}

impl Drop for Metrics {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_frame_times_and_reports_percentiles() {
        let mut metrics = Metrics::default();
        assert_eq!(metrics.redraw_count(), 0);
        metrics.redraw();
        metrics.record(0.016, 1024, 2048);
        metrics.record(0.032, 4096, 512);
        let report = metrics.report();
        assert_eq!(report.frames, 2);
        assert_eq!(report.scene_redraws, 1);
        assert!(report.frame_max_ms > 30.0);
        assert_eq!(report.peak_texture_bytes, 4096);
    }
}
