// Inspired by https://github.com/KillzXGaming/Switch-Toolbox/tree/master/Switch_Toolbox_Library/Animations/AnimationRewrite

use crate::layout::{BuiltPane, Pane};
use std::ops::RangeInclusive;

#[derive(Copy, Clone, Debug)]
pub struct Keyframe {
    slope: f64,
    frame: f64,
    value: f64,
}

impl Keyframe {
    pub const fn new(slope: f64, frame: f64, value: f64) -> Self {
        Self {
            slope,
            frame,
            value,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AnimationTrack {
    keyframes: &'static [Keyframe],
}

impl AnimationTrack {
    pub const fn new(keyframes: &'static [Keyframe]) -> Self {
        Self { keyframes }
    }

    pub const fn duration(&self) -> f64 {
        match self.keyframes.last() {
            Some(frame) => frame.frame,
            None => 0.0,
        }
    }

    pub fn ending_value(&self) -> f64 {
        self.keyframes.last().map_or(0.0, |x| x.value)
    }

    pub fn value_at(&self, frame: f64) -> f64 {
        match self.keyframes {
            [] => return 0.0,
            [keyframe] => return keyframe.value,
            _ => {}
        }

        let mut before = self.keyframes.first().unwrap();
        let mut after = self.keyframes.last().unwrap();
        for keyframe in self.keyframes {
            if keyframe.frame <= frame {
                before = keyframe;
            }
            if keyframe.frame >= frame && keyframe.frame < after.frame {
                after = keyframe;
            }
        }

        if before.frame == after.frame {
            return before.value;
        }

        let diff = frame - before.frame;
        let weight = diff / (after.frame - before.frame);

        let lhs = before.value;
        let rhs = after.value;
        let ls = before.slope;
        let rs = after.slope;

        lhs + (lhs - rhs) * (2.0 * weight - 3.0) * weight * weight
            + (diff * (weight - 1.0)) * (ls * (weight - 1.0) + rs * weight)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AnimationSet<const N: usize> {
    elements: [AnimationSetElement; N],
    duration: u32,
}

impl<const N: usize> AnimationSet<N> {
    pub const fn new(elements: [AnimationSetElement; N]) -> Self {
        // We do love const limitations
        let mut duration = 0;
        let mut i = 0;
        while i < N {
            let animations = elements[i].animations;
            let mut j = 0;
            while j < animations.len() {
                let new_duration = animations[j].1.duration().ceil() as u32;
                if new_duration > duration {
                    duration = new_duration;
                }
                j += 1;
            }
            i += 1;
        }

        Self { elements, duration }
    }

    pub fn animate(&self, origin_pane: &BuiltPane) -> AnimationAnimator<N> {
        AnimationAnimator {
            elements: self.elements.map(|element| {
                let pane = origin_pane
                    .child(element.path)
                    .unwrap_or_else(|| panic!("Missing animation element {:?}", element.path));
                ActiveElement {
                    pane,
                    animations: element.animations,
                }
            }),
            frame_iter: Some(0..=self.duration),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AnimationSetElement {
    path: &'static [&'static str],
    animations: &'static [(AnimatableParameter, AnimationTrack)],
}

impl AnimationSetElement {
    pub const fn new(
        path: &'static [&'static str],
        animations: &'static [(AnimatableParameter, AnimationTrack)],
    ) -> Self {
        Self { path, animations }
    }

    pub const fn to_set(self) -> AnimationSet<1> {
        AnimationSet::new([self])
    }
}

#[derive(Copy, Clone, Debug)]
pub enum AnimatableParameter {
    TranslateX,
    TranslateY,
    Scale,
    ScaleX,
    ScaleY,
    Alpha,
}

impl AnimatableParameter {
    pub fn set_value(&self, pane: &mut Pane, value: f64) {
        match self {
            AnimatableParameter::TranslateX => pane.rect.set_x(value as i32),
            AnimatableParameter::TranslateY => pane.rect.set_y(value as i32),
            AnimatableParameter::Scale => pane.set_scale(value),
            AnimatableParameter::ScaleX => pane.scale.0 = value,
            AnimatableParameter::ScaleY => pane.scale.1 = value,
            AnimatableParameter::Alpha => pane.alpha = value as u8,
        }
    }
}

pub trait ActiveAnimator {
    fn advance_frame(&mut self, new_animators: &mut Vec<Box<dyn ActiveAnimator>>) -> bool;
}

pub struct AnimationAnimator<const N: usize> {
    elements: [ActiveElement; N],
    frame_iter: Option<RangeInclusive<u32>>,
}

struct ActiveElement {
    pane: BuiltPane,
    animations: &'static [(AnimatableParameter, AnimationTrack)],
}

impl<const N: usize> ActiveAnimator for AnimationAnimator<N> {
    fn advance_frame(&mut self, new_animators: &mut Vec<Box<dyn ActiveAnimator>>) -> bool {
        let _ = new_animators;
        let Some(iter) = &mut self.frame_iter else {
            return false;
        };
        if let Some(frame) = iter.next() {
            for element in self.elements.iter() {
                element.pane.edit(|pane| {
                    for (parameter, animation) in element.animations {
                        parameter.set_value(pane, animation.value_at(frame as f64));
                    }
                });
            }
            true
        } else {
            for element in self.elements.iter() {
                element.pane.edit(|pane| {
                    for (parameter, animation) in element.animations {
                        parameter.set_value(pane, animation.ending_value());
                    }
                });
            }
            self.frame_iter = None;
            false
        }
    }
}
