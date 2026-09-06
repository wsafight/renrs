use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let first = image::open(&args[0])?.into_rgba8();
    let last = image::open(&args[1])?.into_rgba8();
    assert_eq!(first.dimensions(), last.dimensions());
    let colors: HashSet<_> = first.pixels().step_by(17).collect();
    let changed = first
        .pixels()
        .zip(last.pixels())
        .filter(|(a, b)| {
            a.0.iter()
                .zip(b.0.iter())
                .take(3)
                .any(|(a, b)| a.abs_diff(*b) > 12)
        })
        .count();
    println!(
        "{}",
        serde_json::json!({"width": first.width(), "height": first.height(), "colors": colors.len(), "changed_pixels": changed})
    );
    Ok(())
}
