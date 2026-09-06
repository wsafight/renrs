use super::text::draw_text;
use macroquad::prelude::*;
use renrs::runtime::{StageState, VisualEffect};
use renrs::{Runtime, WaitState};

use crate::frontend::{FocusAxis, FocusScope, UiAction, UiActions};

use super::app::{App, Screen};
use super::ui_common::{button, choice_button, color, color_alpha, draw_centered, wrap_lines};
use super::{CANVAS_HEIGHT, CANVAS_WIDTH};

impl App {
    pub(super) fn draw_game(&mut self, mouse: Vec2, actions: &UiActions) {
        let stage = self
            .runtime
            .as_ref()
            .map(Runtime::shared_stage)
            .unwrap_or_default();
        self.draw_stage(&stage);
        self.draw_media();
        self.draw_custom_screen(renrs::screens::ScreenKind::Hud, mouse, actions);
        if self.overlay.is_some() {
            return;
        }
        if !self.assets.is_ready()
            || !self.media_ready()
            || self.storage.loading
            || self.storage.quit_after_save
        {
            draw_centered(
                &super::ui_text::tr("Loading..."),
                CANVAS_WIDTH / 2.0,
                CANVAS_HEIGHT / 2.0,
                self.theme.ui_font_size,
                color(&self.theme.text_color),
            );
            return;
        }
        let waiting = self.runtime.as_ref().and_then(Runtime::waiting).cloned();
        let custom = match &waiting {
            Some(WaitState::Dialogue) if !stage.nvl => Some(renrs::screens::ScreenKind::Dialogue),
            Some(WaitState::Choice { .. }) => Some(renrs::screens::ScreenKind::Choices),
            _ => None,
        };
        if let Some(kind) = custom
            && self.draw_custom_screen(kind, mouse, actions)
        {
            return;
        }
        if !matches!(waiting, Some(WaitState::Finished)) {
            self.draw_toolbar(mouse, actions);
            if self.overlay.is_some() {
                return;
            }
        }

        match waiting {
            Some(WaitState::Dialogue) => {
                let previous = self.theme.clone();
                self.theme = self.dialogue_theme();
                let nvl = self.dialogue_view.nvl.clone();
                if let Some(dialogue) = nvl.as_deref().or(stage.dialogue.as_ref()) {
                    self.draw_dialogue(dialogue, mouse, actions);
                }
                self.theme = previous;
            }
            Some(WaitState::Choice { options }) => self.draw_choices(&options, mouse, actions),
            Some(WaitState::Finished) => self.draw_finished(mouse, actions),
            Some(WaitState::Effect {
                effect: VisualEffect::Fade { seconds },
            }) => {
                let alpha = if self.theme.reduced_motion || seconds <= f32::EPSILON {
                    0.0
                } else {
                    (self.effect_remaining / seconds).clamp(0.0, 1.0)
                };
                draw_rectangle(
                    self.theme.layout.toolbar.x,
                    self.theme.layout.toolbar.y + self.theme.layout.toolbar.height,
                    self.theme.layout.toolbar.width,
                    CANVAS_HEIGHT - self.theme.layout.toolbar.y - self.theme.layout.toolbar.height,
                    Color::new(0.0, 0.0, 0.0, alpha),
                );
            }
            Some(
                WaitState::Effect {
                    effect:
                        VisualEffect::Tween { .. }
                        | VisualEffect::Parallel { .. }
                        | VisualEffect::Transform { .. }
                        | VisualEffect::Dissolve { .. }
                        | VisualEffect::Video { .. },
                }
                | WaitState::Pause { .. },
            )
            | None => {}
        }
    }

    pub(super) fn draw_stage(&self, stage: &StageState) {
        if let Some(WaitState::Effect {
            effect:
                VisualEffect::Parallel {
                    from,
                    tracks,
                    seconds,
                },
        }) = self.runtime.as_ref().and_then(Runtime::waiting)
        {
            let elapsed = if self.theme.reduced_motion {
                *seconds
            } else {
                seconds - self.effect_remaining
            };
            self.draw_stage_tinted(
                &renrs_runtime::animation::sample(from, tracks, elapsed),
                1.0,
            );
            return;
        }
        self.draw_stage_tinted(stage, 1.0);
    }

    pub(super) fn draw_stage_tinted(&self, stage: &StageState, opacity: f32) {
        self.draw_stage_on(stage, opacity, self.canvas_target.as_ref());
    }

    pub(super) fn draw_stage_on(
        &self,
        stage: &StageState,
        opacity: f32,
        target: Option<&RenderTarget>,
    ) {
        let camera = self.camera_transform(stage);
        let opacity = opacity * camera.alpha;
        let _camera = super::stage_camera::StageCamera::new(camera, target);
        self.draw_background_tinted(stage.background.as_deref(), opacity);
        let tween = self
            .runtime
            .as_ref()
            .and_then(Runtime::waiting)
            .and_then(|waiting| match waiting {
                WaitState::Effect {
                    effect:
                        VisualEffect::Tween {
                            alias,
                            from,
                            to,
                            seconds,
                        },
                } => Some((alias.as_str(), *from, *to, *seconds)),
                _ => None,
            });
        let animated_transform =
            self.runtime
                .as_ref()
                .and_then(Runtime::waiting)
                .and_then(|waiting| match waiting {
                    WaitState::Effect {
                        effect:
                            VisualEffect::Transform {
                                alias,
                                from,
                                to,
                                seconds,
                                easing,
                            },
                    } => Some((alias.as_str(), *from, *to, *seconds, *easing)),
                    _ => None,
                });
        let mut sprites = stage.sprites.iter().enumerate().collect::<Vec<_>>();
        sprites.sort_by_key(|(index, sprite)| (sprite.layer, *index));
        for (_, sprite) in sprites {
            let texture = self.assets.textures.get(&sprite.path);
            let Some(dimensions) = sprite
                .composition
                .as_ref()
                .map(|image| (image.width as f32, image.height as f32))
                .or_else(|| texture.map(|texture| (texture.width(), texture.height())))
            else {
                continue;
            };
            let transform = if let Some((alias, from, to, seconds, easing)) = animated_transform
                && alias == sprite.alias
            {
                let progress = if seconds <= f32::EPSILON {
                    1.0
                } else {
                    1.0 - (self.effect_remaining / seconds).clamp(0.0, 1.0)
                };
                from.interpolate(to, easing.sample(progress))
            } else {
                sprite.transform
            };
            let source_width = transform.crop.map_or(dimensions.0, |crop| crop.width);
            let source_height = transform.crop.map_or(dimensions.1, |crop| crop.height);
            let target_height = 650.0 * transform.scale;
            let target_width = source_width / source_height * target_height;
            let x = if let Some((alias, from, to, seconds)) = tween
                && alias == sprite.alias
            {
                let progress = if seconds <= f32::EPSILON {
                    1.0
                } else {
                    1.0 - (self.effect_remaining / seconds).clamp(0.0, 1.0)
                };
                let start = sprite_x(from, target_width);
                let end = sprite_x(to, target_width);
                start + (end - start) * progress
            } else {
                sprite_x(sprite.position, target_width)
            } + transform.x
                - transform.anchor_x * target_width;
            let anchor = vec2(
                x + transform.anchor_x * target_width,
                CANVAS_HEIGHT + transform.y,
            );
            let y = anchor.y - transform.anchor_y * target_height;
            if let Some(composition) = &sprite.composition {
                self.draw_image_layers(
                    composition,
                    transform,
                    super::image_layers::Placement {
                        position: vec2(x, y),
                        size: vec2(target_width, target_height),
                        pivot: anchor,
                        opacity,
                    },
                );
                continue;
            }
            draw_texture_ex(
                texture.unwrap(),
                x,
                y,
                Color::new(1.0, 1.0, 1.0, transform.alpha * opacity),
                DrawTextureParams {
                    dest_size: Some(vec2(target_width, target_height)),
                    source: transform
                        .crop
                        .map(|crop| Rect::new(crop.x, crop.y, crop.width, crop.height)),
                    rotation: transform.rotation.to_radians(),
                    pivot: Some(anchor),
                    ..Default::default()
                },
            );
        }
    }

    pub(super) fn draw_background(&self, path: Option<&str>) {
        self.draw_background_tinted(path, 1.0);
    }

    fn draw_background_tinted(&self, path: Option<&str>, opacity: f32) {
        if path.is_none() || opacity >= 1.0 {
            draw_rectangle(
                0.0,
                0.0,
                CANVAS_WIDTH,
                CANVAS_HEIGHT,
                color_alpha(&self.theme.background_color, opacity),
            );
        }
        if let Some(texture) = path.and_then(|path| self.assets.textures.get(path)) {
            let scale = (CANVAS_WIDTH / texture.width()).max(CANVAS_HEIGHT / texture.height());
            let width = texture.width() * scale;
            let height = texture.height() * scale;
            draw_texture_ex(
                texture,
                (CANVAS_WIDTH - width) / 2.0,
                (CANVAS_HEIGHT - height) / 2.0,
                Color::new(1.0, 1.0, 1.0, opacity),
                DrawTextureParams {
                    dest_size: Some(vec2(width, height)),
                    ..Default::default()
                },
            );
        }
    }

    pub(super) fn draw_dialogue(
        &mut self,
        dialogue: &renrs::runtime::DialogueState,
        mouse: Vec2,
        actions: &UiActions,
    ) {
        let layout = self.theme.layout.dialogue_rect;
        let panel = Rect::new(layout.x, layout.y, layout.width, layout.height);
        draw_rectangle(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            color_alpha(&self.theme.panel_color, 0.97),
        );
        draw_rectangle(
            panel.x,
            panel.y,
            6.0,
            panel.h,
            color(&self.theme.accent_color),
        );
        if let Some(name) = &dialogue.speaker_name {
            draw_text(
                name,
                panel.x + 34.0,
                panel.y + 42.0,
                f32::from(self.theme.dialogue_font_size),
                color(&dialogue.speaker_color),
            );
        }

        self.dialogue_view.prepare(dialogue, &self.theme);
        let count = self.dialogue_view.character_count();
        let visible = (self.visible_characters as usize).min(count);
        self.dialogue_view.draw(visible, &self.theme);
        let advance = panel.contains(mouse) && is_mouse_button_pressed(MouseButton::Left)
            || (self.focus.selected().is_none() && actions.pressed(UiAction::Activate));
        if advance {
            if visible < count {
                self.visible_characters = count as f32;
            } else {
                self.continue_story();
            }
        }
    }

    pub(super) fn draw_choices(&mut self, options: &[String], mouse: Vec2, actions: &UiActions) {
        if self.focus.selected().is_none() && actions.pressed(UiAction::Up) {
            self.selected_choice = self.selected_choice.saturating_sub(1);
        }
        if self.focus.selected().is_none() && actions.pressed(UiAction::Down) {
            self.selected_choice = (self.selected_choice + 1).min(options.len().saturating_sub(1));
        }
        if self.focus.selected().is_none() {
            if actions.pressed(UiAction::Home) {
                self.selected_choice = 0;
            }
            if actions.pressed(UiAction::End) {
                self.selected_choice = options.len().saturating_sub(1);
            }
            self.selected_choice = self
                .selected_choice
                .saturating_add_signed(-mouse_wheel().1.round() as isize)
                .min(options.len().saturating_sub(1));
        }
        let rows = 5.min(options.len());
        let start = self.selected_choice.saturating_sub(rows.saturating_sub(1));
        let heights: Vec<_> = options
            .iter()
            .skip(start)
            .take(rows)
            .map(|option| {
                let lines = wrap_lines(option, 676.0, self.theme.ui_font_size).len();
                (lines as f32 * f32::from(self.theme.ui_font_size + 4) + 16.0).clamp(52.0, 92.0)
            })
            .collect();
        let total_height = heights.iter().sum::<f32>() + rows.saturating_sub(1) as f32 * 14.0;
        let mut y = (CANVAS_HEIGHT - total_height) / 2.0 + 20.0;
        let mut selected = None;
        for ((index, option), height) in options
            .iter()
            .enumerate()
            .skip(start)
            .take(rows)
            .zip(heights)
        {
            let rect = Rect::new(290.0, y, 700.0, height);
            y += height + 14.0;
            if choice_button(
                rect,
                option,
                mouse,
                index == self.selected_choice,
                &self.theme,
            ) {
                selected = Some(index);
            }
        }
        if options.len() > rows {
            draw_centered(
                &format!("{}-{} / {}", start + 1, start + rows, options.len()),
                CANVAS_WIDTH / 2.0,
                y + 16.0,
                18,
                color(&self.theme.text_color),
            );
        }
        if self.focus.selected().is_none() && actions.pressed(UiAction::Activate) {
            selected = Some(self.selected_choice);
        }
        if let Some(index) = selected {
            let result = self
                .runtime
                .as_mut()
                .expect("playing requires runtime")
                .choose(index);
            self.handle_wait(result);
        }
    }

    pub(super) fn draw_finished(&mut self, mouse: Vec2, actions: &UiActions) {
        self.focus.update(
            FocusScope::Finished,
            1,
            Some(0),
            FocusAxis::Vertical,
            actions,
        );
        draw_rectangle(
            0.0,
            0.0,
            CANVAS_WIDTH,
            CANVAS_HEIGHT,
            color_alpha(&self.theme.panel_color, 0.9),
        );
        draw_centered(
            &super::ui_text::tr("The End"),
            640.0,
            290.0,
            self.theme.title_font_size.saturating_sub(10),
            color(&self.theme.text_color),
        );
        if button(
            Rect::new(470.0, 346.0, 340.0, 52.0),
            "Return to title",
            mouse,
            true,
            true,
            actions.pressed(UiAction::Activate) || actions.pressed(UiAction::Back),
            &self.theme,
        ) {
            self.screen = Screen::MainMenu;
            self.runtime = None;
            self.audio.stop_music();
            self.audio.stop_voice();
        }
    }
}

fn sprite_x(position: renrs::syntax::Position, width: f32) -> f32 {
    match position {
        renrs::syntax::Position::Left => 84.0,
        renrs::syntax::Position::Center => (CANVAS_WIDTH - width) / 2.0,
        renrs::syntax::Position::Right => CANVAS_WIDTH - width - 84.0,
    }
}
