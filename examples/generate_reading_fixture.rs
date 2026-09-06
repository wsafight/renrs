use std::{fs, path::PathBuf, process::Command};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: generate_reading_fixture <new-directory>")?;
    renrs::scaffold::create_project(&root, "Signal Journal", "org.renrs.reading")?;
    fs::write(root.join("screens.json"), "{}")?;
    fs::write(root.join("resources.json"), r#"{"exclude":["source.mp4"]}"#)?;
    let status = Command::new(std::env::var_os("RENRS_FFMPEG").unwrap_or_else(|| "ffmpeg".into()))
        .args([
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=24",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "4",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ])
        .arg(root.join("source.mp4"))
        .status()?;
    if !status.success() {
        return Err("fixture video generation failed".into());
    }
    let converter = std::env::current_exe()?
        .parent()
        .ok_or("missing executable directory")?
        .parent()
        .ok_or("missing build directory")?
        .join(format!("renrs-video{}", std::env::consts::EXE_SUFFIX));
    for (name, stream) in [("clips/frames", false), ("clips/stream", true)] {
        let mut command = Command::new(&converter);
        command.arg(root.join("source.mp4")).arg(&root).arg(name);
        if stream {
            command.arg("--stream");
        }
        if !command.status()?.success() {
            return Err("fixture video conversion failed".into());
        }
    }
    fs::write(
        root.join("script.rns"),
        r##"config title "Signal Journal"
config id "org.renrs.reading"
define guide = character "Mira" color "#F6C85F"
label start:
    scene "images/studio.png"
    show "images/mira.png" as left at left
    show "images/mira.png" as right at right
    transform left scale 0.6 over 0
    transform right scale 0.6 over 0
    @id "begin" guide "The journal is ready."
    parallel:
        timeline:
            transform left x 200 over 1.5
        timeline:
            pause 0.25
            transform right x -180 alpha 0.65 over 1.25
    nvl on
    @id "journal_one" guide "{ruby=signal}Signal{/ruby} received."
    @id "journal_two" guide "{u}Two frequencies{/u}, one conversation."
    nvl clear
    @id "journal_three" guide "A new page."
    nvl off
    @id "media" menu:
        "Frame soundtrack":
            video "clips/frames/clip.json" over 4
            jump ending
        "Stream soundtrack":
            video "clips/stream/clip.json" over 4
            jump ending
label ending:
    @id "done" guide "Playback complete."
    @id "finish" return
"##,
    )?;
    fs::write(
        root.join("routes.json"),
        r#"{"routes":[{"name":"frames","choices":[0],"expect_label":"ending"},{"name":"stream","choices":[1],"expect_label":"ending"}]}"#,
    )?;
    renrs::ProjectSource::open(&root)?
        .compile()
        .map_err(|errors| {
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    println!("{}", root.display());
    Ok(())
}
