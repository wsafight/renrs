use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("output directory required")?,
    );
    renrs::scaffold::create_project(&root, "Travel Journal", "org.renrs.composition")?;
    fs::write(
        root.join("script.rns"),
        r#"config title "Travel Journal"
config id "org.renrs.composition"
default inventory = list("Map", "Key", "Radio")
default selected = ""
default packed = ""
default coat = true
default reward = 0
label start:
    scene "images/studio.png"
    show "images/mira.layers.json" as mira
    "The journey begins."
    transform camera scale 1.2 x -60 over 0.6 ease in_out
    menu:
        "Depart":
            jump departure
        "Wait":
            jump departure
label departure:
    "The selected item is {selected}."
    "The travel bag contains {packed}."
    return
"#,
    )?;
    fs::write(
        root.join("screens.json"),
        include_bytes!("composable_screens.json"),
    )?;
    fs::create_dir_all(root.join("extensions"))?;
    fs::write(root.join("extensions/reward.rhai"), "input + 1")?;
    fs::write(
        root.join("extensions.json"),
        br#"{"version":1,"modules":{"reward":"extensions/reward.rhai"}}"#,
    )?;
    let image = image::open(root.join("images/mira.png"))?.to_rgba8();
    let (width, height) = image.dimensions();
    let mut body = image.clone();
    let mut face = image.clone();
    for (x, y, pixel) in image.enumerate_pixels() {
        if y < 259 {
            body.put_pixel(x, y, image::Rgba([0; 4]));
        }
        if y >= 260 {
            face.put_pixel(x, y, image::Rgba([0; 4]));
        }
        let _ = pixel;
    }
    let mut blink = face.clone();
    for x in 170..250 {
        for y in 167..182 {
            if (x < 190 || x > 230) && x < width && y < height {
                blink.put_pixel(
                    x,
                    y,
                    image::Rgba(if (174..=175).contains(&y) {
                        [70, 84, 86, 255]
                    } else {
                        [232, 183, 155, 255]
                    }),
                );
            }
        }
    }
    body.save(root.join("images/body.png"))?;
    face.save(root.join("images/face.png"))?;
    blink.save(root.join("images/blink.png"))?;
    fs::write(
        root.join("images/mira.layers.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "width":width,"height":height,"layers":[
                {"path":"images/body.png","when":"coat"},
                {"path":"images/face.png","frames":[{"path":"images/face.png","seconds":2.5},{"path":"images/blink.png","seconds":0.16}]}
            ]
        }))?,
    )?;
    fs::write(root.join("routes.json"), br#"{"routes":[]}"#)?;
    renrs::ProjectSource::open(&root)?
        .compile()
        .map_err(|errors| format!("{errors:?}"))?;
    println!("{}", root.display());
    Ok(())
}
