use std::path::PathBuf;

use macroquad::prelude::*;
use renrs::compiler::Program;
use renrs::theme::{self, Theme as ProjectTheme};
use renrs::{ProjectSource, user_data_directory};

use crate::frontend::{UiAction, UiActions};

use super::app::App;
use super::options::Options;
use super::smoke::SmokeTest;
use super::text::{Face, draw_text};
use super::ui_common::{color, wrap_lines};
use super::{CANVAS_HEIGHT, CANVAS_WIDTH};

pub(crate) fn window_conf() -> macroquad::conf::Conf {
    let options = Options::read();
    let mut native = Conf {
        window_title: "RenRS".to_owned(),
        window_width: options.window.0,
        window_height: options.window.1,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    };
    native.platform.blocking_event_loop = cfg!(target_os = "macos");
    macroquad::conf::Conf {
        miniquad_conf: native,
        update_on: Some(macroquad::conf::UpdateTrigger {
            key_down: true,
            mouse_down: true,
            mouse_up: true,
            mouse_motion: true,
            mouse_wheel: true,
            touch: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(crate) async fn run() {
    let options = Options::read();
    let benchmark = options.benchmark.map(super::benchmark::BenchmarkRun::new);
    let smoke = options
        .smoke_test
        .map(|output| SmokeTest::new(output).unwrap_or_else(fail));
    let source = match ProjectSource::open(options.project) {
        Ok(source) => source,
        Err(error) => {
            if smoke.is_some() {
                return fail(error.to_string());
            }
            run_error_screen(vec![error.to_string()]).await;
            return;
        }
    };
    let (theme, theme_notice) = match load_theme(&source) {
        Ok(theme) => (theme, None),
        Err(error) => (
            ProjectTheme::default(),
            Some(format!("Theme ignored: {error}")),
        ),
    };
    let font_notice = install_project_font(&source, &theme);
    let initial_notice = [theme_notice, font_notice]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("; ");
    let initial_notice = (!initial_notice.is_empty()).then_some(initial_notice);
    match load_program(&source) {
        Ok(program) => {
            let data_root = match smoke.as_ref().map_or_else(
                || user_data_directory(&program.project_id),
                |test| Ok(test.data_root()),
            ) {
                Ok(path) => path,
                Err(error) => {
                    run_error_screen(vec![error.to_string()]).await;
                    return;
                }
            };
            let data_root = options
                .profile
                .as_ref()
                .or_else(|| benchmark.as_ref().map(|run| &run.output))
                .map_or(data_root, |path| path.with_extension("data"));
            run_player(
                source,
                program,
                theme,
                data_root,
                initial_notice,
                smoke,
                options.profile,
                benchmark,
            )
            .await;
        }
        Err(errors) => {
            if smoke.is_some() {
                return fail(errors.join("\n"));
            }
            run_error_screen(errors).await;
        }
    }
}

pub(super) fn load_theme(source: &ProjectSource) -> Result<ProjectTheme, theme::ThemeError> {
    if !source.contains(theme::THEME_FILE) {
        return Ok(ProjectTheme::default());
    }
    let bytes = source
        .read(theme::THEME_FILE)
        .map_err(|source_error| theme::ThemeError::Read {
            path: theme::THEME_FILE.to_owned(),
            source: std::io::Error::other(source_error),
        })?;
    ProjectTheme::from_slice(&bytes, theme::THEME_FILE)
}

pub(super) fn install_project_font(source: &ProjectSource, theme: &ProjectTheme) -> Option<String> {
    let family = theme
        .font_paths()
        .map(|path| source.read(path).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .and_then(Face::family);
    match family {
        Ok(faces) => {
            super::text::install_family(faces);
            None
        }
        Err(error) => {
            super::text::install_family(Face::defaults());
            Some(format!("Could not load font family: {error}"))
        }
    }
}

pub(super) fn load_program(source: &ProjectSource) -> Result<Program, Vec<String>> {
    source.compile().map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|item| item.to_string())
            .collect()
    })
}

async fn run_error_screen(errors: Vec<String>) {
    let mut wake = super::wake::WakeTimer::new();
    loop {
        let frame_started = std::time::Instant::now();
        clear_background(color("#111318"));
        draw_rectangle(0.0, 0.0, screen_width(), 72.0, color("#d45845"));
        draw_text("RenRS could not start this game", 40.0, 48.0, 30.0, WHITE);
        let mut y = 116.0;
        for error in &errors {
            for line in wrap_lines(error, screen_width() - 80.0, 20) {
                draw_text(&line, 40.0, y, 20.0, color("#f5f1ea"));
                y += 30.0;
            }
            y += 12.0;
        }
        if is_key_pressed(KeyCode::Escape) || is_quit_requested() {
            super::text::shutdown();
            break;
        }
        wake.schedule(false, frame_started);
        next_frame().await;
    }
}

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
async fn run_player(
    source: ProjectSource,
    program: Program,
    theme: ProjectTheme,
    data_root: PathBuf,
    initial_notice: Option<String>,
    mut smoke: Option<SmokeTest>,
    profile: Option<PathBuf>,
    mut benchmark: Option<super::benchmark::BenchmarkRun>,
) {
    let mut app = App::new(source, program, theme, &data_root);
    app.storage.profile_path = data_root.join("profile.json");
    match std::fs::read(&app.storage.profile_path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(profile) => app.storage.profile = profile,
            Err(error) => {
                app.storage.profile_path.clear();
                app.notice = Some((
                    format!("Profile persistence disabled; original file preserved: {error}"),
                    8.0,
                ));
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            app.storage.profile_path.clear();
            app.notice = Some((
                format!("Profile persistence disabled; original file preserved: {error}"),
                8.0,
            ));
        }
    }
    if let Some(notice) = initial_notice {
        let notice = app.notice.take().map_or(notice.clone(), |(existing, _)| {
            format!("{notice}; {existing}")
        });
        app.notice = Some((notice, 6.0));
    }
    if smoke.is_some() {
        SmokeTest::initialize(&mut app).unwrap_or_else(fail);
    }
    if profile.is_some() {
        app.storage.quiet = true;
        app.settings.music_volume = 0.0;
        app.settings.sound_volume = 0.0;
        app.settings.voice_volume = 0.0;
        app.settings.text_speed = 100.0;
        app.settings.auto_delay = 0.5;
        app.start_new_game();
        app.playback.auto = true;
    }
    if benchmark.is_some() {
        app.storage.quiet = true;
        app.settings.music_volume = 0.0;
        app.settings.sound_volume = 0.0;
        app.settings.voice_volume = 0.0;
        app.start_new_game();
        app.visible_characters = f32::MAX;
    }
    let watcher = app
        .source
        .watch_root()
        .map(|_| super::reload_worker::ReloadWorker::new(app.source.clone()));
    let canvas_target = render_target(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32);
    canvas_target.texture.set_filter(FilterMode::Linear);
    let mut canvas_camera =
        Camera2D::from_display_rect(Rect::new(0.0, 0.0, CANVAS_WIDTH, CANVAS_HEIGHT));
    canvas_camera.render_target = Some(canvas_target.clone());
    app.canvas_target = Some(canvas_target.clone());
    let transition_target = render_target(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32);
    transition_target.texture.set_filter(FilterMode::Linear);
    app.transition_target = Some(transition_target);

    prevent_quit();
    let mut wake = super::wake::WakeTimer::new();
    let mut previous_pointer = (mouse_position(), screen_width(), screen_height());
    let mut profile_frames = 0;
    while !app.quit {
        let frame_started = std::time::Instant::now();
        profile_frames += 1;
        if profile.is_none() || profile_frames > 60 {
            app.metrics.record(
                get_frame_time(),
                app.assets.resident_bytes(),
                app.assets.queued_bytes(),
            );
        }
        app.update_storage();
        if is_quit_requested() {
            app.request_quit();
        }
        if let Some(active) = &watcher
            && let Some(result) = active.poll()
            && let Err(error) = result.and_then(|bundle| {
                let generation = bundle.generation;
                app.reload_project(bundle)?;
                active.acknowledge(generation);
                Ok(())
            })
        {
            app.notice = Some((format!("Reload failed: {error}"), 5.0));
            app.redraw = true;
        }
        let smoke_actions = smoke
            .as_mut()
            .map(|test| test.prepare(&mut app).unwrap_or_else(fail));
        app.prepare_assets();
        if smoke.is_none() {
            app.update_timers();
        }
        if profile.is_some()
            && let Some(runtime) = &mut app.runtime
            && matches!(runtime.waiting(), Some(renrs::WaitState::Choice { .. }))
        {
            let result = runtime.choose(0);
            app.handle_wait(result);
        }
        if let Some(path) = &profile
            && profile_frames >= 360
        {
            renrs::storage::write_json(path, &app.metrics.report()).unwrap_or_else(fail);
            println!("Profile recorded: {}", path.display());
            app.quit = true;
        }

        let scale = (screen_width() / CANVAS_WIDTH).min(screen_height() / CANVAS_HEIGHT);
        let offset = vec2(
            (screen_width() - CANVAS_WIDTH * scale) * 0.5,
            (screen_height() - CANVAS_HEIGHT * scale) * 0.5,
        );
        let mouse = if smoke.is_some() || benchmark.is_some() {
            vec2(-100.0, -100.0)
        } else {
            (Vec2::from(mouse_position()) - offset) / scale
        };
        let actions = if benchmark.is_some() {
            UiActions::new(Vec::new())
        } else {
            smoke_actions.unwrap_or_else(collect_ui_actions)
        };
        let pointer = (mouse_position(), screen_width(), screen_height());
        let input = benchmark.is_none()
            && (pointer != previous_pointer
                || !get_keys_pressed().is_empty()
                || !get_keys_released().is_empty()
                || !get_keys_down().is_empty()
                || is_mouse_button_down(MouseButton::Left)
                || is_mouse_button_released(MouseButton::Left)
                || is_mouse_button_pressed(MouseButton::Right)
                || mouse_wheel() != (0.0, 0.0));
        previous_pointer = pointer;
        let redraw = app.redraw || input || app.animated() || smoke.is_some() || profile.is_some();
        if redraw {
            app.redraw = false;
            set_camera(&canvas_camera);
            clear_background(BLACK);
            app.draw(mouse, &actions);
            app.metrics.redraw();
            // UI actions are processed during drawing; show their result on the next wake.
            app.redraw |= input;
        }
        set_default_camera();
        clear_background(BLACK);
        draw_texture_ex(
            &canvas_target.texture,
            offset.x,
            offset.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(CANVAS_WIDTH * scale, CANVAS_HEIGHT * scale)),
                flip_y: true,
                ..Default::default()
            },
        );

        let events = app
            .runtime
            .as_mut()
            .map_or_else(Vec::new, |runtime| runtime.drain_audio_events().collect());
        let completed = app.audio.handle(events, &app.source, &app.settings);
        if let Some(runtime) = &mut app.runtime {
            if completed.music {
                runtime.complete_music_track();
            }
            if completed.sound {
                runtime.complete_sound_track();
            }
        }
        if let Some(test) = &mut smoke {
            app.quit = test.after_draw(&app).unwrap_or_else(fail);
        }
        if let Some(run) = &mut benchmark {
            app.quit = run.after_draw(&app).unwrap_or_else(fail);
        }
        wake.schedule(
            app.animated() || app.redraw || smoke.is_some() || profile.is_some(),
            frame_started,
        );
        next_frame().await;
    }
    if smoke.as_ref().is_some_and(|test| !test.finished()) {
        return fail("smoke test interrupted before all captures completed");
    }
    if app.settings_dirty {
        let _ = app.settings.save(&app.settings_path);
    }
    if app.read_dirty {
        let _ = app.read_state.save(&app.read_path);
    }
    super::text::shutdown();
}

fn fail<T>(error: impl std::fmt::Display) -> T {
    eprintln!("error: {error}");
    std::process::exit(1);
}

fn collect_ui_actions() -> UiActions {
    let mut actions = Vec::new();
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    if is_key_pressed(KeyCode::Tab) {
        actions.push(if shift {
            UiAction::FocusPrevious
        } else {
            UiAction::FocusNext
        });
    }
    for (pressed, action) in [
        (
            is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space),
            UiAction::Activate,
        ),
        (is_key_pressed(KeyCode::Escape), UiAction::Back),
        (is_key_pressed(KeyCode::Up), UiAction::Up),
        (is_key_pressed(KeyCode::Down), UiAction::Down),
        (is_key_pressed(KeyCode::Left), UiAction::Left),
        (is_key_pressed(KeyCode::Right), UiAction::Right),
        (is_key_pressed(KeyCode::S), UiAction::QuickSave),
        (is_key_pressed(KeyCode::L), UiAction::QuickLoad),
        (is_key_pressed(KeyCode::H), UiAction::OpenHistory),
        (is_key_pressed(KeyCode::PageUp), UiAction::PageUp),
        (is_key_pressed(KeyCode::PageDown), UiAction::PageDown),
        (is_key_pressed(KeyCode::Home), UiAction::Home),
        (is_key_pressed(KeyCode::End), UiAction::End),
    ] {
        if pressed {
            actions.push(action);
        }
    }
    UiActions::new(actions)
}
