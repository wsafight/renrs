use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: generate_product_fixture <new-directory>")?;
    renrs::scaffold::create_project(&destination, "Signal Studio", "org.renrs.product")?;
    fs::write(
        destination.join("script.rns"),
        r##"config title "Signal Studio"
config id "org.renrs.product"
default player_name = "Reader"
default trust = 0
default large = 9007199254740993
default bag = list("key")
define guide = character "Mira" color "#F6C85F"
label start:
    scene "images/studio.png"
    show "images/mira.png" as mira at right
    @id "opening" guide "Welcome {player_name}. The {b}receiver{/b} has a {color=#77d5bb}signal{/color}.{br}Choose a frequency."
    @id "route" menu:
        "Listen":
            set trust = 1
            jump ending
        "Reply":
            set trust = 2
            jump ending
label ending:
    scene "images/rooftop.png"
    @id "ending" guide "We found the frequency. {player_name}: {trust}."
    @id "finish" return
"##,
    )?;
    fs::write(
        destination.join("screens.json"),
        include_str!("product_screens.json"),
    )?;
    fs::write(
        destination.join("routes.json"),
        r#"{"routes":[{"name":"listen","choices":[0],"expect_label":"ending","expect_variables":{"trust":1}},{"name":"reply","choices":[1],"expect_label":"ending","expect_variables":{"trust":2}}]}"#,
    )?;
    renrs::ProjectSource::open(&destination)?
        .compile()
        .map_err(|errors| {
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    println!("{}", destination.display());
    Ok(())
}
