#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

#[path = "../src/player/text.rs"]
mod text;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "RenRS text cache verification".to_owned(),
        window_width: 800,
        window_height: 600,
        high_dpi: true,
        ..Default::default()
    }
}

fn draw_sample() {
    text::draw_text("AV To gyp 0123456789", 42.0, 110.0, 32.0, WHITE);
    text::draw_text(
        "\u{4e2d}\u{6587}\u{5b57}\u{4f53}\u{9a8c}\u{8bc1}",
        42.0,
        180.0,
        32.0,
        WHITE,
    );
    text::draw_text("han", 42.0, 230.0, 16.0, WHITE);
    text::draw_text("\u{6c49}", 42.0, 260.0, 32.0, WHITE);
    draw_line(42.0, 267.0, 74.0, 267.0, 1.5, WHITE);
}

fn begin_frame(camera: &Camera2D) {
    set_camera(camera);
    clear_background(BLACK);
}

fn capture(target: &RenderTarget) -> Image {
    set_default_camera();
    clear_background(BLACK);
    draw_texture_ex(
        &target.texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(800.0, 600.0)),
            flip_y: true,
            ..Default::default()
        },
    );
    get_screen_data()
}

#[macroquad::main(window_conf)]
async fn main() {
    let output = std::path::PathBuf::from(std::env::args().nth(1).expect("new output directory"));
    std::fs::create_dir(&output).unwrap();
    let face = text::Face::from_bytes(std::fs::read("assets/fonts/NotoSansSC-Medium.ttf").unwrap())
        .unwrap();
    text::install_family(vec![face]);
    assert_eq!(text::stats()["cached_glyphs"], 0);
    assert!(text::measure_width("AV To", 32) > 0.0);
    assert_eq!(text::stats()["atlas_bytes"], 0);
    let target = render_target(800, 600);
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 800.0, 600.0));
    camera.render_target = Some(target.clone());
    begin_frame(&camera);
    draw_sample();
    let reference = capture(&target);
    reference.export_png(output.join("reference.png").to_str().unwrap());
    next_frame().await;

    begin_frame(&camera);
    draw_sample();
    // Force multiple atlas recycles after visible quads have already been queued.
    let unseen: String = (0x4e00..0x5000).filter_map(char::from_u32).collect();
    text::draw_text(unseen, -100_000.0, -1000.0, 60.0, WHITE);
    let recycled = capture(&target);
    recycled.export_png(output.join("recycled.png").to_str().unwrap());
    assert_eq!(
        reference.bytes, recycled.bytes,
        "atlas reuse damaged queued text"
    );
    assert!(text::stats()["atlas_resets"].as_u64().unwrap() > 1);
    assert_eq!(text::stats()["atlas_bytes"], 4 * 1024 * 1024);
    next_frame().await;

    begin_frame(&camera);
    text::install_family(text::Face::family(vec![]).unwrap());
    assert_eq!(text::stats()["cached_glyphs"], 0);
    draw_sample();
    let reloaded = capture(&target);
    reloaded.export_png(output.join("reloaded.png").to_str().unwrap());
    assert_eq!(
        reference.bytes, reloaded.bytes,
        "font reload changed glyph positions"
    );
    std::fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"passed": true, "text_cache": text::stats()}),
        )
        .unwrap(),
    )
    .unwrap();
    next_frame().await;
    text::shutdown();
    println!("Text cache verification passed: {}", output.display());
}
