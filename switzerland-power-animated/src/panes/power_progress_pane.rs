use crate::Result;
use crate::alignment::Alignment;
use crate::animation::AnimatableParameter::{Alpha, Scale, ScaleX, ScaleY, TranslateX, TranslateY};
use crate::animation::{AnimationSet, AnimationSetElement, AnimationTrack, Keyframe};
use crate::font::FontSet;
use crate::generator::{HEIGHT, WIDTH};
use crate::layout::{BuiltPane, Pane, PaneContents, TextPaneContents};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::BlendMode;
use std::rc::Rc;

pub const WIN_COLOR: Color = Color::RGB(233, 255, 0);
pub const LOSE_COLOR: Color = Color::RGB(43, 24, 255);

pub fn power_progress_pane(
    bold_font: Rc<FontSet<'static>>,
    bold_font_small: Rc<FontSet<'static>>,
    swiss_flag: PaneContents,
) -> Result<BuiltPane> {
    let win_lose_background =
        PaneContents::image_png(include_bytes!("images/win-lose-background.png"))?;
    Ok(Pane {
        rect: Rect::new(0, 0, WIDTH, HEIGHT),
        children: vec![
            Pane {
                rect: Rect::new(0, 0, 1000, 700),
                contents: PaneContents::image_png(include_bytes!(
                    "images/power-progress-background.png"
                ))?,
                contents_blending: BlendMode::None,
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                rect: Rect::new(0, 264, 68, 68),
                contents: swiss_flag,
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                name: "set_outcome_pane",
                rect: Rect::new(0, -79, 30, 40),
                children: vec![
                    Pane {
                        name: "set_score_text",
                        rect: Rect::new(0, 191, 300, 300),
                        scale: (1.0, 0.5),
                        alpha: 0,
                        anchor: Alignment::BOTTOM,
                        contents: PaneContents::Text(
                            TextPaneContents::new("3 - 2", &bold_font)
                                .scale(0.7, 0.7)
                                .alignment(Alignment::BOTTOM),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    win_lose_pane(-207, 147, &bold_font_small, &win_lose_background)?,
                    win_lose_pane(0, 147, &bold_font_small, &win_lose_background)?,
                    win_lose_pane(207, 147, &bold_font_small, &win_lose_background)?,
                    win_lose_pane(-207, 122, &bold_font_small, &win_lose_background)?,
                    win_lose_pane(0, 122, &bold_font_small, &win_lose_background)?,
                ],
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                name: "power_pane",
                rect: Rect::new(0, -140, 30, 40),
                children: vec![
                    Pane {
                        name: "power_text",
                        rect: Rect::new(-425, 97, 600, 76),
                        anchor: Alignment::LEFT,
                        contents: PaneContents::Text(
                            TextPaneContents::new("Power", &bold_font)
                                .scale(0.5, 0.5)
                                .secondary_scale(0.8)
                                .alignment(Alignment::LEFT),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "power_value_text",
                        rect: Rect::new(0, -22, 700, 150),
                        contents: PaneContents::Text(
                            TextPaneContents::new("1500.0", &bold_font).secondary_scale(0.8),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "power_diff",
                        rect: Rect::new(341, 87, 158, 163),
                        scale: (1.19, 1.19),
                        alpha: 0,
                        children: vec![
                            Pane {
                                name: "image_container",
                                children: vec![
                                    Pane {
                                        rect: Rect::new(0, 12, 256, 225),
                                        scale: (0.6, 0.6),
                                        contents: PaneContents::image_png(include_bytes!(
                                            "images/power-diff-background.png"
                                        ))?,
                                        ..Pane::EMPTY
                                    }
                                    .build(),
                                ],
                                ..Pane::EMPTY
                            }
                            .build(),
                            Pane {
                                name: "value",
                                rect: Rect::new(0, 10, 105, 40),
                                scale: (0.6, 0.6),
                                contents: PaneContents::Text(
                                    TextPaneContents::new("+100.0", &bold_font_small)
                                        .scale(0.7, 0.7),
                                ),
                                ..Pane::EMPTY
                            }
                            .build(),
                        ],
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "point_diff_anim",
                        rect: Rect::new(311, 76, 200, 76),
                        alpha: 0,
                        contents: PaneContents::Text(
                            TextPaneContents::new("+100.0", &bold_font)
                                .scale(0.5, 0.5)
                                .secondary_scale(0.8),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                ],
                ..Pane::EMPTY
            }
            .build(),
        ],
        ..Pane::EMPTY
    }
    .build())
}

fn win_lose_pane(
    x: i32,
    y: i32,
    font: &Rc<FontSet<'static>>,
    background: &PaneContents,
) -> Result<BuiltPane> {
    Ok(Pane {
        name: "win_lose_pane",
        rect: Rect::new(x, y, 30, 40),
        children: vec![
            Pane {
                name: "animation_pane",
                scale: (0.5, 0.5),
                alpha: 0,
                children: vec![
                    Pane {
                        rect: Rect::new(0, 0, 500, 128),
                        scale: (0.4, 0.4),
                        contents: background.clone(),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "text",
                        rect: Rect::new(0, 0, 166, 46),
                        contents: PaneContents::Text(
                            TextPaneContents::new("WIN", font)
                                .color(WIN_COLOR)
                                .scale(0.9, 0.9),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                ],
                ..Pane::EMPTY
            }
            .build(),
        ],
        ..Pane::EMPTY
    }
    .build())
}

pub const WINDOW_IN: AnimationSet<1> = AnimationSetElement::new(
    &[],
    &[
        (
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.000129002059, 0.0, 0.8),
                Keyframe::new(0.0143402768, 8.0, 1.07803428),
                Keyframe::new(-0.0130057139, 12.0, 0.97208333),
                Keyframe::new(0.0, 14.0, 1.0),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 0.0), Keyframe::new(0.0, 6.0, 255.0)]),
        ),
    ],
)
.to_set();

pub const WIN_LOSE_POSITIONS: [[(i32, i32); 5]; 4] = [
    [(-104, 147), (104, 147), (207, 147), (-207, 122), (0, 122)],
    [(-207, 147), (0, 147), (207, 147), (-207, 122), (0, 122)],
    [(-103, 172), (104, 172), (-103, 122), (104, 122), (0, 122)],
    [(-207, 172), (0, 172), (207, 172), (-207, 122), (0, 122)],
];

pub const WIN_LOSE_IN: AnimationSet<1> = AnimationSetElement::new(
    &[],
    &[
        (
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.175000012, 0.0, 0.5),
                Keyframe::new(0.0833333358, 4.0, 1.2),
                Keyframe::new(-0.100000024, 6.0, 1.0),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 0.0), Keyframe::new(0.0, 2.0, 255.0)]),
        ),
    ],
)
.to_set();

pub const SET_SCORE_IN: AnimationSet<1> = AnimationSetElement::new(
    &["set_outcome_pane", "set_score_text"],
    &[
        (ScaleX, AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 1.0)])),
        (
            ScaleY,
            AnimationTrack::new(&[
                Keyframe::new(0.193693876, 0.0, 0.5),
                Keyframe::new(0.07790083, 4.0, 1.27477551),
                Keyframe::new(-0.0686938763, 6.0, 0.967404962),
                Keyframe::new(0.01629752, 8.0, 1.0),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 0.0), Keyframe::new(0.0, 4.0, 255.0)]),
        ),
    ],
)
.to_set();

pub const POWER_DIFF_IN: AnimationSet<3> = AnimationSet::new([
    AnimationSetElement::new(
        &["power_pane", "power_diff"],
        &[(
            Alpha,
            AnimationTrack::new(&[
                Keyframe::new(31.875, 0.0, 0.0),
                Keyframe::new(0.674603164, 8.0, 255.0),
            ]),
        )],
    ),
    AnimationSetElement::new(
        &["power_pane", "power_diff", "image_container"],
        &[
            (
                Scale,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 0.0, 1.52646542),
                    Keyframe::new(0.0, 2.0, 2.0),
                    Keyframe::new(0.0, 8.0, 1.0),
                ]),
            ),
            (
                Alpha,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 0.0, 3.0),
                    Keyframe::new(0.0, 8.0, 255.0),
                ]),
            ),
        ],
    ),
    AnimationSetElement::new(
        &["power_pane", "power_diff", "value"],
        &[
            (
                Scale,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 0.0, 1.43733871),
                    Keyframe::new(-0.05466734, 6.0, 0.7287192),
                    Keyframe::new(0.0, 8.0, 1.0),
                ]),
            ),
            (
                Alpha,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 0.0, 0.0),
                    Keyframe::new(0.0, 8.0, 255.0),
                ]),
            ),
        ],
    ),
]);

pub const POWER_ADD: AnimationSet<2> = AnimationSet::new([
    AnimationSetElement::new(
        &["power_pane", "point_diff_anim"],
        &[
            (
                TranslateX,
                AnimationTrack::new(&[
                    Keyframe::new(-14.6, 0.0, 311.0),
                    Keyframe::new(-14.6, 20.0, 19.0),
                ]),
            ),
            (
                TranslateY,
                AnimationTrack::new(&[
                    Keyframe::new(3.05107236, 0.0, 76.0),
                    Keyframe::new(0.5562192, 6.0, 94.3064346),
                    Keyframe::new(-6.09331656, 13.0, 83.23085),
                    Keyframe::new(-10.6044073, 20.0, 9.0),
                ]),
            ),
            (
                Alpha,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 0.0, 0.0),
                    Keyframe::new(0.0, 2.0, 255.0),
                    Keyframe::new(0.0, 17.0, 255.0),
                    Keyframe::new(0.0, 20.0, 0.0),
                ]),
            ),
        ],
    ),
    AnimationSetElement::new(
        &["power_pane", "power_value_text"],
        &[
            (
                TranslateY,
                AnimationTrack::new(&[
                    Keyframe::new(-6.33333349, 20.0, -22.0),
                    Keyframe::new(-6.33333349, 23.0, -41.0),
                    Keyframe::new(10.0, 23.0, -41.0),
                    Keyframe::new(0.0, 25.0, -21.0),
                    Keyframe::new(0.0, 26.0, -22.0),
                ]),
            ),
            (
                ScaleX,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 20.0, 1.0),
                    Keyframe::new(-0.00681583071, 20.0, 1.0),
                    Keyframe::new(-0.00681583071, 23.0, 0.9795525),
                    Keyframe::new(0.0127114058, 23.0, 0.9795525),
                    Keyframe::new(0.0, 25.0, 1.00497532),
                    Keyframe::new(0.0, 26.0, 1.0),
                ]),
            ),
            (
                ScaleY,
                AnimationTrack::new(&[
                    Keyframe::new(0.0, 20.0, 1.0),
                    Keyframe::new(-0.07112324, 20.0, 1.0),
                    Keyframe::new(-0.07112324, 23.0, 0.7866303),
                    Keyframe::new(0.109172523, 23.0, 0.7866303),
                    Keyframe::new(0.0, 25.0, 1.00497532),
                    Keyframe::new(0.0, 26.0, 1.0),
                ]),
            ),
        ],
    ),
]);

pub const WINDOW_OUT: AnimationSet<1> = AnimationSetElement::new(
    &[],
    &[
        (
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.0, 0.0, 1.0),
                Keyframe::new(0.0, 6.0, 1.0666821),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 255.0), Keyframe::new(0.0, 6.0, 0.0)]),
        ),
    ],
)
.to_set();
