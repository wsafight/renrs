use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let destination = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: generate_visual_fixture <new-directory>")?;
    renrs::scaffold::create_project(&destination, "Signal at Dawn", "org.renrs.visual")?;
    let mut script = String::from(
        "config title \"Signal at Dawn\"\nconfig id \"org.renrs.visual\"\ndefault trust = 0\ndefault player_name = \"Reader\"\ndefine guide = character \"Mira\" color \"#F6C85F\"\nlabel start:\n    scene \"images/studio.png\"\n    show \"images/mira.png\" as mira at right\n",
    );
    let long = "The receiver carried a distant voice across the city. We noted the time, checked the frequency, and waited for the next reply. ".repeat(10);
    let _ = writeln!(
        script,
        "    @id \"opening\" guide {}",
        serde_json::json!(long)
    );
    for index in 1..=36 {
        let _ = writeln!(
            script,
            "    guide \"Record {index}: the rooftops light up as another signal reaches the station.\""
        );
    }
    script.push_str("    menu:\n");
    for index in 1..=12 {
        let _ = writeln!(
            script,
            "        \"Station {index}: follow the voice across the river and ask the night operator whether anyone has heard the same three notes before\":\n            set trust = {index}\n            jump ending"
        );
    }
    script.push_str("label ending:\n    scene \"images/rooftop.png\"\n    guide \"We found the right frequency.\"\n    return\n");
    fs::write(destination.join("script.rns"), script).map_err(|error| error.to_string())?;
    fs::write(destination.join("routes.json"), br#"{"routes":[{"name":"first","choices":[0],"expect_label":"ending","expect_variables":{"trust":1}},{"name":"last","choices":[11],"expect_label":"ending","expect_variables":{"trust":12}}]}"#).map_err(|error| error.to_string())?;
    fs::write(
        destination.join("screens.json"),
        include_str!("visual_screens.json"),
    )
    .map_err(|error| error.to_string())?;
    renrs::ProjectSource::open(&destination)
        .map_err(|error| error.to_string())?
        .compile()
        .map_err(|errors| {
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    println!("Generated visual fixture at {}", destination.display());
    Ok(())
}
