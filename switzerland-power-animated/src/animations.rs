use crate::animation::{AnimationTrack, Keyframe};

pub const WINDOW_IN_SCALE: AnimationTrack<&[Keyframe]> = AnimationTrack::new(&[
    Keyframe::new(0.0, -70.0, 1.23238242),
    Keyframe::new(0.0, 5.0, 1.23238242),
    Keyframe::new(-0.000431764929, 5.0, 1.2),
    Keyframe::new(-0.0395297222, 5.0, 1.2),
    Keyframe::new(-0.0395297222, 11.0, 0.9628217),
    Keyframe::new(0.0192887783, 11.0, 0.9628217),
    Keyframe::new(0.00413092, 14.0, 1.020688),
    Keyframe::new(0.0, 20.0, 1.0),
]);

pub const WINDOW_IN_ALPHA: AnimationTrack<&[Keyframe]> = AnimationTrack::new(&[
    Keyframe::new(0.0, 5.0, 0.0),
    Keyframe::new(0.0, 13.0, 255.0),
]);

pub const PROGRESS_IN_SCALE: AnimationTrack<&[Keyframe]> = AnimationTrack::new(&[
    Keyframe::new(0.1633873, 0.0, 0.4347826),
    Keyframe::new(0.0, 5.0, 1.25171912),
    Keyframe::new(0.0, 9.0, 1.0),
]);

pub const PROGRESS_IN_ALPHA: AnimationTrack<&[Keyframe]> =
    AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 0.0), Keyframe::new(0.0, 3.0, 255.0)]);

pub const WINDOW_OUT_SCALE: AnimationTrack<&[Keyframe]> = AnimationTrack::new(&[
    Keyframe::new(0.04875219, 0.0, 1.0),
    Keyframe::new(0.0, 10.0, 1.23238242),
]);

pub const WINDOW_OUT_ALPHA: AnimationTrack<&[Keyframe]> = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 255.0),
    Keyframe::new(0.0, 10.0, 0.0),
]);
