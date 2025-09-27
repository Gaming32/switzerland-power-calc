use crate::Result;
use crate::alignment::Alignment;
use crate::font::FontSet;
use crate::generator::{HEIGHT, WIDTH};
use crate::layout::{BuiltPane, Pane, PaneContents};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::rc::Rc;
use crate::animation::{AnimationTrack, Keyframe};

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
                        contents: PaneContents::Text {
                            text: "3 - 2".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (0.7, 0.7),
                            text_alignment: Alignment::BOTTOM,
                        },
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
                        contents: PaneContents::Text {
                            text: "Power".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (0.5, 0.5),
                            text_alignment: Alignment::LEFT,
                        },
                        ..Pane::EMPTY
                    }.build(),
                    Pane {
                        name: "power_value_text",
                        rect: Rect::new(0, -22, 700, 150),
                        contents: PaneContents::Text {
                            text: "1500.0".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (1.0, 1.0),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }.build(),
                    Pane {
                        name: "power_diff",
                        rect: Rect::new(341, 87, 158, 163),
                        scale: (1.19, 1.19),
                        children: vec![
                            Pane {
                                name: "image",
                                rect: Rect::new(0, 12, 256, 225),
                                scale: (0.6, 0.6),
                                contents: PaneContents::image_png(include_bytes!("images/power-diff-background.png"))?,
                                ..Pane::EMPTY
                            }.build(),
                            Pane {
                                name: "value",
                                rect: Rect::new(0, 10, 105, 40),
                                scale: (0.6, 0.6),
                                contents: PaneContents::Text {
                                    text: "+100.0".into(),
                                    font: bold_font_small.clone(),
                                    color: Color::WHITE,
                                    scale: (0.7, 0.7),
                                    text_alignment: Alignment::CENTER,
                                },
                                ..Pane::EMPTY
                            }.build(),
                        ],
                        ..Pane::EMPTY
                    }.build(),
                ],
                ..Pane::EMPTY
            }.build(),
        ],
        ..Pane::EMPTY
    }
    .build())
}

fn win_lose_pane(x: i32, y: i32, font: &Rc<FontSet<'static>>, background: &PaneContents) -> Result<BuiltPane> {
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
                    }.build(),
                    Pane {
                        name: "text",
                        rect: Rect::new(0, 0, 166, 46),
                        contents: PaneContents::Text {
                            text: "WIN".into(),
                            font: font.clone(),
                            color: WIN_COLOR,
                            scale: (0.9, 0.9),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }.build(),
                ],
                ..Pane::EMPTY
            }.build(),
        ],
        ..Pane::EMPTY
    }.build())
}

pub const WINDOW_IN_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.000129002059, 0.0, 0.8),
    Keyframe::new(0.0143402768, 8.0, 1.07803428),
    Keyframe::new(-0.0130057139, 12.0, 0.97208333),
    Keyframe::new(0.0, 14.0, 1.0),
]);

pub const WINDOW_IN_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 0.0),
    Keyframe::new(0.0, 6.0, 255.0),
]);

pub const WIN_LOSE_IN_SCALE: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.175000012, 0.0, 0.5),
    Keyframe::new(0.0833333358, 4.0, 1.2),
    Keyframe::new(-0.100000024, 6.0, 1.0),
]);

pub const WIN_LOSE_IN_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 0.0),
    Keyframe::new(0.0, 2.0, 255.0),
]);

pub const WIN_LOSE_POSITIONS: [[(i32, i32); 5]; 4] = [
    [(-104, 147), (104, 147), (207, 147), (-207, 122), (0, 122)],
    [(-207, 147), (0, 147), (207, 147), (-207, 122), (0, 122)],
    [(-103, 172), (104, 172), (-103, 122), (104, 122), (0, 122)],
    [(-207, 172), (0, 172), (207, 172), (-207, 122), (0, 122)],
];

pub const SET_SCORE_IN_SCALE_X: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 1.0),
]);

pub const SET_SCORE_IN_SCALE_Y: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.193693876, 0.0, 0.5),
    Keyframe::new(0.07790083, 4.0, 1.27477551),
    Keyframe::new(-0.0686938763, 6.0, 0.967404962),
    Keyframe::new(0.01629752, 8.0, 1.0),
]);

pub const SET_SCORE_IN_ALPHA: AnimationTrack = AnimationTrack::new(&[
    Keyframe::new(0.0, 0.0, 0.0),
    Keyframe::new(0.0, 4.0, 255.0),
]);
