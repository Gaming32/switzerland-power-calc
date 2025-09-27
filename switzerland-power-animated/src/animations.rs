use crate::animation::{AnimationTrack, Keyframe};

pub const WINDOW_IN_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, -70.0, 1.23238242),
    Keyframe::new(0.0, 5.0, 1.23238242),
    Keyframe::new(-0.000431764929, 5.0, 1.2),
    Keyframe::new(-0.0395297222, 5.0, 1.2),
    Keyframe::new(-0.0395297222, 11.0, 0.9628217),
    Keyframe::new(0.0192887783, 11.0, 0.9628217),
    Keyframe::new(0.00413092, 14.0, 1.020688),
    Keyframe::new(0.0, 20.0, 1.0),
]);

pub const WINDOW_IN_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 5.0, 0.0),
    Keyframe::new(0.0, 13.0, 255.0),
]);

pub const PROGRESS_IN_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.1633873, 0.0, 0.4347826),
    Keyframe::new(0.0, 5.0, 1.25171912),
    Keyframe::new(0.0, 9.0, 1.0),
]);

pub const PROGRESS_IN_ALPHA: AnimationTrack =
    AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 0.0), Keyframe::new(0.0, 3.0, 255.0)]);

pub const RESULT_PROGRESS_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(-0.0700630844, 0.0, 1.0),
    Keyframe::new(-0.0005248937, 3.0, 0.905991733),
]);

pub const RESULT_PROGRESS_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(-85.0, 0.0, 255.0),
    Keyframe::new(-85.0, 3.0, 0.0),
]);

pub const RESULT_PANE_ALPHA: AnimationTrack =
    AnimationTrack::new(&[Keyframe::new(0.0, 2.0, 0.0), Keyframe::new(0.0, 6.0, 255.0)]);

pub const RESULT_POWER_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0354311466, 3.0, 1.0),
    Keyframe::new(-0.006858194, 6.0, 1.10629344),
    Keyframe::new(-0.0702922046, 8.0, 0.965709031),
    Keyframe::new(0.03429097, 8.0, 0.965709031),
    Keyframe::new(0.0, 9.0, 1.0),
]);

pub const RESULT_RANK_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 2.0),
    Keyframe::new(0.0, 10.0, 0.8074777),
    Keyframe::new(0.0, 13.0, 1.062221),
    Keyframe::new(-0.0311105251, 15.0, 1.0),
]);

pub const RESULT_RANK_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 0.0),
    Keyframe::new(0.0, 15.0, 255.0),
]);

pub const WINDOW_OUT_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.04875219, 0.0, 1.0),
    Keyframe::new(0.0, 10.0, 1.23238242),
]);

pub const WINDOW_OUT_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 255.0),
    Keyframe::new(0.0, 10.0, 0.0),
]);
