// Inspired by https://github.com/KillzXGaming/Switch-Toolbox/tree/master/Switch_Toolbox_Library/Animations/AnimationRewrite

use crate::Result;
use crate::layout::{BuiltPane, Pane};

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

    pub fn duration(&self) -> f64 {
        self.keyframes.last().map_or(0.0, |x| x.frame)
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
}

impl<const N: usize> AnimationSet<N> {
    pub const fn new(elements: [AnimationSetElement; N]) -> Self {
        Self { elements }
    }

    pub fn animate(
        &self,
        render_pane: &BuiltPane,
        origin_pane: &BuiltPane,
        end_delay: u32,
        mut render_frame: impl FnMut(&BuiltPane, u32) -> Result<()>,
    ) -> Result<()> {
        struct ResolvedElement {
            pane: BuiltPane,
            animations: &'static [(AnimatableParameter, AnimationTrack)],
        }
        let elements = self.elements.map(|element| {
            let pane = origin_pane
                .child(element.path)
                .unwrap_or_else(|| panic!("Missing animation element {:?}", element.path));
            ResolvedElement {
                pane,
                animations: element.animations,
            }
        });
        let duration = elements
            .iter()
            .flat_map(|x| x.animations)
            .map(|(_, anim)| anim.duration().ceil() as u32)
            .max()
            .expect("No elements in AnimationSet");

        for frame in 0..=duration {
            for element in elements.iter() {
                element.pane.edit(|pane| {
                    for (parameter, animation) in element.animations {
                        parameter.set_value(pane, animation.value_at(frame as f64));
                    }
                });
            }
            render_frame(render_pane, 1)?;
        }

        for element in elements.iter() {
            element.pane.edit(|pane| {
                for (parameter, animation) in element.animations {
                    parameter.set_value(pane, animation.ending_value());
                }
            });
        }
        render_frame(render_pane, end_delay)?;

        Ok(())
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
    Scale,
    XScale,
    YScale,
    Alpha,
}

impl AnimatableParameter {
    pub fn set_value(&self, pane: &mut Pane, value: f64) {
        match self {
            AnimatableParameter::Scale => pane.set_scale(value),
            AnimatableParameter::XScale => pane.scale.0 = value,
            AnimatableParameter::YScale => pane.scale.1 = value,
            AnimatableParameter::Alpha => pane.alpha = value as u8,
        }
    }
}
