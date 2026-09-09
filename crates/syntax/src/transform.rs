use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionKind {
    Fade,
    Dissolve,
    PushLeft,
    PushRight,
    WipeLeft,
    WipeRight,
    PunchH,
    PunchV,
}

impl TransitionKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fade => "fade",
            Self::Dissolve => "dissolve",
            Self::PushLeft => "push left",
            Self::PushRight => "push right",
            Self::WipeLeft => "wipe left",
            Self::WipeRight => "wipe right",
            Self::PunchH => "punch h",
            Self::PunchV => "punch v",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformState {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rotation: f32,
    pub alpha: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub crop: Option<CropRect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xalign: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yalign: Option<f32>,
}

impl Default for TransformState {
    fn default() -> Self {
        Self::identity()
    }
}

impl TransformState {
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            alpha: 1.0,
            anchor_x: 0.0,
            anchor_y: 1.0,
            crop: None,
            xalign: None,
            yalign: None,
        }
    }

    #[must_use]
    pub fn interpolate(self, target: Self, progress: f32) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        Self {
            x: lerp(self.x, target.x, progress),
            y: lerp(self.y, target.y, progress),
            scale: lerp(self.scale, target.scale, progress),
            rotation: lerp(self.rotation, target.rotation, progress),
            alpha: lerp(self.alpha, target.alpha, progress),
            anchor_x: lerp(self.anchor_x, target.anchor_x, progress),
            anchor_y: lerp(self.anchor_y, target.anchor_y, progress),
            crop: match (self.crop, target.crop) {
                (Some(from), Some(to)) => Some(from.interpolate(to, progress)),
                (from, to) => {
                    if progress < 1.0 {
                        from
                    } else {
                        to
                    }
                }
            },
            xalign: interpolate_align(self.xalign, target.xalign, progress),
            yalign: interpolate_align(self.yalign, target.yalign, progress),
        }
    }
}

fn interpolate_align(from: Option<f32>, to: Option<f32>, progress: f32) -> Option<f32> {
    match (from, to) {
        (Some(from), Some(to)) => Some(lerp(from, to, progress)),
        (from, _) if progress < 1.0 => from,
        (_, to) => to,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TransformProperties {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub scale: Option<f32>,
    pub rotation: Option<f32>,
    pub alpha: Option<f32>,
    pub anchor: Option<(f32, f32)>,
    pub crop: Option<Option<CropRect>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xalign: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yalign: Option<f32>,
}

impl TransformProperties {
    #[must_use]
    pub fn apply(self, mut state: TransformState) -> TransformState {
        if let Some(value) = self.x {
            state.x = value;
        }
        if let Some(value) = self.y {
            state.y = value;
        }
        if let Some(value) = self.scale {
            state.scale = value;
        }
        if let Some(value) = self.rotation {
            state.rotation = value;
        }
        if let Some(value) = self.alpha {
            state.alpha = value;
        }
        if let Some((x, y)) = self.anchor {
            state.anchor_x = x;
            state.anchor_y = y;
        }
        if let Some(value) = self.crop {
            state.crop = value;
        }
        if let Some(value) = self.xalign {
            state.xalign = Some(value);
        }
        if let Some(value) = self.yalign {
            state.yalign = Some(value);
        }
        state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl CropRect {
    pub(crate) fn interpolate(self, target: Self, progress: f32) -> Self {
        Self {
            x: lerp(self.x, target.x, progress),
            y: lerp(self.y, target.y, progress),
            width: lerp(self.width, target.width, progress),
            height: lerp(self.height, target.height, progress),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    #[must_use]
    pub fn sample(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => progress,
            Self::EaseIn => progress * progress,
            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
            Self::EaseInOut => 1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0,
        }
    }
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_numeric_fields_and_clamps_progress() {
        let mut from = TransformState::identity();
        from.crop = Some(CropRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        });
        let to = TransformProperties {
            x: Some(10.0),
            y: Some(-4.0),
            scale: Some(2.0),
            rotation: Some(90.0),
            alpha: Some(0.0),
            anchor: Some((0.5, 0.5)),
            crop: Some(Some(CropRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 40.0,
            })),
            xalign: Some(1.0),
            yalign: Some(0.0),
        }
        .apply(from);
        let mid = from.interpolate(to, 0.5);
        assert!((mid.x - 5.0).abs() < f32::EPSILON);
        assert!((mid.scale - 1.5).abs() < f32::EPSILON);
        assert!((mid.crop.unwrap().height - 20.0).abs() < f32::EPSILON);
        assert_eq!(from.interpolate(to, -1.0), from);
        assert_eq!(from.interpolate(to, 2.0), to);
    }

    #[test]
    fn crop_and_align_hold_the_source_until_the_end() {
        let from = TransformState::identity();
        let mut to = from;
        to.crop = Some(CropRect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        });
        to.xalign = Some(1.0);
        let mid = from.interpolate(to, 0.5);
        assert_eq!(mid.crop, None);
        assert_eq!(mid.xalign, None);
        assert_eq!(from.interpolate(to, 1.0).crop, to.crop);
        assert_eq!(from.interpolate(to, 1.0).xalign, Some(1.0));
    }

    #[test]
    fn easing_samples_and_transition_names_are_stable() {
        assert!((Easing::Linear.sample(0.25) - 0.25).abs() < f32::EPSILON);
        assert!((Easing::EaseIn.sample(0.5) - 0.25).abs() < f32::EPSILON);
        assert!((Easing::EaseOut.sample(0.5) - 0.75).abs() < f32::EPSILON);
        assert!((Easing::EaseInOut.sample(0.25) - 0.125).abs() < f32::EPSILON);
        assert!((Easing::Linear.sample(-1.0)).abs() < f32::EPSILON);
        assert!((Easing::Linear.sample(2.0) - 1.0).abs() < f32::EPSILON);
        assert_eq!(TransitionKind::Fade.name(), "fade");
        assert_eq!(TransitionKind::PushLeft.name(), "push left");
        assert_eq!(TransitionKind::WipeRight.name(), "wipe right");
        assert_eq!(TransitionKind::PunchV.name(), "punch v");
    }
}
