use super::app::App;
use macroquad::prelude::get_screen_data;
use std::path::PathBuf;
use std::time::Instant;

pub(super) struct BenchmarkRun {
    pub(super) output: PathBuf,
    started: Instant,
    ready: Option<Instant>,
    measuring: Option<Instant>,
    redraws: usize,
    intervals: Vec<f64>,
    previous: Option<Instant>,
    capturing: Option<Instant>,
}

impl BenchmarkRun {
    pub(super) fn new(output: PathBuf) -> Self {
        Self {
            output,
            started: Instant::now(),
            ready: None,
            measuring: None,
            redraws: 0,
            intervals: Vec::new(),
            previous: None,
            capturing: None,
        }
    }
    pub(super) fn after_draw(&mut self, app: &App) -> Result<bool, String> {
        if let Some(started) = self.capturing {
            if started.elapsed().as_secs_f64() < 0.5 {
                return Ok(false);
            }
            self.capture("end.png")?;
            println!("Benchmark recorded: {}", self.output.display());
            return Ok(true);
        }
        if let Some(error) = &app.fatal_error {
            return Err(error.clone());
        }
        if let Some((notice, _)) = &app.notice {
            return Err(notice.clone());
        }
        if self.started.elapsed().as_secs() > 30 {
            return Err("benchmark timed out".to_owned());
        }
        if !app.assets.is_ready() || !app.media_ready() {
            return Ok(false);
        }
        let now = Instant::now();
        let ready = *self.ready.get_or_insert(now);
        if self.measuring.is_none() {
            if ready.elapsed().as_secs_f64() < 2.0 {
                return Ok(false);
            }
            renrs::storage::write_json(&self.output.with_extension("ready.json"), &serde_json::json!({
                "pid": std::process::id(), "ready_ms": (ready - self.started).as_secs_f64() * 1000.0,
                "warmup_seconds": 2, "measurement_seconds": 8,
            })).map_err(|error| error.to_string())?;
            self.redraws = app.metrics.redraw_count();
            self.measuring = Some(Instant::now());
            self.previous = Some(Instant::now());
            return Ok(false);
        }
        if let Some(previous) = self.previous.replace(now) {
            self.intervals.push((now - previous).as_secs_f64() * 1000.0);
        }
        let elapsed = self.measuring.unwrap().elapsed().as_secs_f64();
        if elapsed < 8.0 {
            return Ok(false);
        }
        self.intervals.sort_by(f64::total_cmp);
        let percentile =
            |p: f64| self.intervals[((self.intervals.len() - 1) as f64 * p).ceil() as usize];
        renrs::storage::write_json(&self.output, &serde_json::json!({
            "engine": "RenRS", "elapsed_seconds": elapsed,
            "scene_redraws": app.metrics.redraw_count() - self.redraws,
            "wake_count": self.intervals.len(), "wake_p50_ms": percentile(0.5), "wake_p95_ms": percentile(0.95),
            "texture_bytes": app.assets.resident_bytes(), "waiting": app.runtime.as_ref().and_then(renrs::Runtime::waiting),
            "text_cache": super::text::stats(),
        })).map_err(|error| error.to_string())?;
        // Readback/PNG allocation happens after the process measurement window.
        self.capture("start.png")?;
        self.capturing = Some(Instant::now());
        Ok(false)
    }

    fn capture(&self, extension: &str) -> Result<(), String> {
        let frame = get_screen_data();
        let image = image::RgbaImage::from_raw(
            u32::from(frame.width),
            u32::from(frame.height),
            frame.bytes,
        )
        .ok_or("invalid framebuffer")?;
        image::imageops::flip_vertical(&image)
            .save(self.output.with_extension(extension))
            .map_err(|error| error.to_string())
    }
}
