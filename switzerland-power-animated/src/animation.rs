// Inspired by https://github.com/KillzXGaming/Switch-Toolbox/tree/master/Switch_Toolbox_Library/Animations/AnimationRewrite

#[derive(Copy, Clone, Debug)]
pub struct Keyframe {
    pub slope: f64,
    pub frame: f64,
    pub value: f64,
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

#[derive(Clone, Debug)]
pub struct AnimationTrack<KF> {
    pub keyframes: KF,
}

impl<KF: AsRef<[Keyframe]>> AnimationTrack<KF> {
    pub const fn new(keyframes: KF) -> Self {
        Self { keyframes }
    }

    pub fn duration(&self) -> f64 {
        self.keyframes.as_ref().last().map_or(0.0, |x| x.frame)
    }

    pub fn value_at(&self, frame: f64) -> f64 {
        let keyframes = self.keyframes.as_ref();

        if keyframes.is_empty() {
            return 0.0;
        }
        if keyframes.len() == 1 {
            return keyframes[0].value;
        }

        let mut before = keyframes.first().unwrap();
        let mut after = keyframes.last().unwrap();
        for keyframe in keyframes {
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
