use std::path::PathBuf;
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    run_from(std::env::args_os().skip(1).collect())
}

fn run_from(mut arguments: Vec<std::ffi::OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let streaming = arguments.last().is_some_and(|arg| arg == "--stream");
    if streaming {
        arguments.pop();
    }
    let [input, root, relative] = arguments.as_slice() else {
        return Err("usage: renrs-video <input-video> <project-root> <clips/name> [--stream] (requires ffmpeg and ffprobe)".into());
    };
    let relative = relative.to_str().ok_or("invalid destination")?;
    if !renrs::resources::visible_path(relative) {
        return Err("unsafe clip destination".into());
    }
    let destination = PathBuf::from(root).join(relative);
    if destination.exists() {
        return Err("clip destination exists".into());
    }
    let parent = destination.parent().ok_or("invalid destination")?;
    std::fs::create_dir_all(parent)?;
    let temporary = tempfile::tempdir_in(parent)?;
    if streaming {
        let mut clip = stream_clip(input, relative, temporary.path())?;
        clip.audio = soundtrack(input, relative, temporary.path(), clip.duration())?;
        std::fs::write(
            temporary.path().join("clip.json"),
            serde_json::to_vec(&clip)?,
        )?;
        std::fs::rename(temporary.path(), &destination)?;
        println!("video \"{relative}/clip.json\" over {:.3}", clip.duration());
        return Ok(());
    }
    let ffmpeg = std::env::var_os("RENRS_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let status = Command::new(ffmpeg)
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(input)
        .args(["-an", "-vf", "fps=24,scale=640:-2", "-frames:v", "7200"])
        .arg(temporary.path().join("%06d.png"))
        .status()?;
    if !status.success() {
        return Err("ffmpeg video conversion failed".into());
    }
    let mut frames: Vec<_> = std::fs::read_dir(temporary.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect();
    frames.sort();
    if frames.is_empty() || frames.len() >= 7200 {
        return Err("clip must have 1..7199 frames; split longer videos".into());
    }
    let mut clip = renrs::video::VideoClip {
        version: 1,
        stream: None,
        audio: None,
        audio_tracks: Vec::new(),
        subtitles: Vec::new(),
        fps: 24.0,
        frames: frames
            .iter()
            .map(|path| {
                format!(
                    "{relative}/{}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            })
            .collect(),
    };
    clip.audio = soundtrack(input, relative, temporary.path(), clip.duration())?;
    std::fs::write(
        temporary.path().join("clip.json"),
        serde_json::to_vec(&clip)?,
    )?;
    std::fs::rename(temporary.path(), &destination)?;
    println!(
        "video \"{relative}/clip.json\" over {:.3}",
        f64::from(u32::try_from(clip.frames.len())?) / 24.0
    );
    Ok(())
}

fn stream_clip(
    input: &std::ffi::OsStr,
    relative: &str,
    output: &std::path::Path,
) -> Result<renrs::video::VideoClip, Box<dyn std::error::Error>> {
    let video = output.join("video.mp4");
    let status = Command::new(std::env::var_os("RENRS_FFMPEG").unwrap_or_else(|| "ffmpeg".into()))
        .args(["-nostdin", "-v", "error", "-i"]).arg(input)
        .args(["-an", "-vf", "fps=24,scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-movflags", "+faststart"])
        .arg(&video).status()?;
    if !status.success() {
        return Err("ffmpeg stream conversion failed".into());
    }
    let metadata =
        Command::new(std::env::var_os("RENRS_FFPROBE").unwrap_or_else(|| "ffprobe".into()))
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "json",
            ])
            .arg(&video)
            .output()?;
    if !metadata.status.success() {
        return Err("ffprobe stream inspection failed".into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout)?;
    let seconds: f32 = metadata["format"]["duration"]
        .as_str()
        .ok_or("missing video duration")?
        .parse()?;
    let clip = renrs::video::VideoClip {
        version: 2,
        fps: 24.0,
        frames: Vec::new(),
        audio: None,
        audio_tracks: Vec::new(),
        subtitles: Vec::new(),
        stream: Some(renrs::video::VideoStream {
            path: format!("{relative}/video.mp4"),
            seconds,
            width: 1280,
            height: 720,
        }),
    };
    Ok(renrs::video::VideoClip::from_slice(&serde_json::to_vec(
        &clip,
    )?)?)
}

fn soundtrack(
    input: &std::ffi::OsStr,
    relative: &str,
    output: &std::path::Path,
    seconds: f32,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let metadata =
        Command::new(std::env::var_os("RENRS_FFPROBE").unwrap_or_else(|| "ffprobe".into()))
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=index",
                "-of",
                "json",
            ])
            .arg(input)
            .output()?;
    if !metadata.status.success() {
        return Err("ffprobe soundtrack inspection failed".into());
    }
    let data: serde_json::Value = serde_json::from_slice(&metadata.stdout)?;
    if data["streams"].as_array().is_none_or(Vec::is_empty) {
        return Ok(None);
    }
    let status = Command::new(std::env::var_os("RENRS_FFMPEG").unwrap_or_else(|| "ffmpeg".into()))
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(input)
        .args([
            "-map",
            "0:a:0",
            "-vn",
            "-af",
            "apad",
            "-t",
            &seconds.to_string(),
            "-ar",
            "48000",
            "-ac",
            "2",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(output.join("audio.wav"))
        .status()?;
    if !status.success() {
        return Err("ffmpeg soundtrack conversion failed".into());
    }
    Ok(Some(format!("{relative}/audio.wav")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_usage_unsafe_paths_and_existing_destinations() {
        assert!(run_from(Vec::new()).is_err());
        let root = tempfile::tempdir().unwrap();
        assert!(
            run_from(vec![
                "clip.mp4".into(),
                root.path().into(),
                "../secret".into()
            ])
            .is_err()
        );
        let dest = root.path().join("clips/name");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(
            run_from(vec![
                "clip.mp4".into(),
                root.path().into(),
                "clips/name".into()
            ])
            .is_err()
        );
    }
}
