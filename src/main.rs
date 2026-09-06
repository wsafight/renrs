#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

mod frontend;
mod player;
mod text_layout;

fn window_conf() -> macroquad::conf::Conf {
    player::window_conf()
}

#[macroquad::main(window_conf)]
async fn main() {
    player::run().await;
}
