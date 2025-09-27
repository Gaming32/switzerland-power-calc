use crate::Result;
use crate::alignment::Alignment;
use crate::animation::{AnimationTrack, Keyframe};
use crate::font::FontSet;
use crate::layout::{BuiltPane, Pane, PaneContents};
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::image::ImageRWops;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::rwops::RWops;
use std::rc::Rc;

pub fn calc_rank_pane(bold_font: Rc<FontSet<'static>>) -> Result<BuiltPane> {
    Ok(Pane {
        rect: Rect::new(0, 0, crate::generator::WIDTH, crate::generator::HEIGHT),
        children: vec![
            Pane {
                rect: Rect::new(0, 0, 1015, 630),
                contents: PaneContents::Image(
                    RWops::from_bytes(include_bytes!("../assets/calc-rank-background.png"))?
                        .load_png()?,
                ),
                ..Pane::EMPTY
            }
            .into(),
            Pane {
                rect: Rect::new(0, 190, 68, 68),
                contents: PaneContents::Image(
                    RWops::from_bytes(include_bytes!("../assets/swiss-flag.png"))?.load_png()?,
                ),
                ..Pane::EMPTY
            }
            .into(),
            Pane {
                rect: Rect::new(0, 2, 811, 10),
                contents: PaneContents::Custom(&|canvas, rect| {
                    for i in 0..101 {
                        let x = rect.x + i * 8;
                        // WARNING: filled_circle takes in ABGR instead of RGBA
                        canvas.filled_circle(
                            x as i16 + 4,
                            rect.y as i16 + 4,
                            2,
                            (64, 255, 255, 255),
                        )?;
                    }
                    Ok(())
                }),
                ..Pane::EMPTY
            }
            .into(),
            Pane {
                name: "progress_pane",
                children: vec![
                    Pane {
                        name: "calculating_text",
                        rect: Rect::new(0, 90, 850, 147),
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (0.6, 0.59),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                    Pane {
                        name: "progress_text",
                        rect: Rect::new(-54, -108, 200, 206),
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (1.2, 1.19),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                    Pane {
                        name: "total_text",
                        rect: Rect::new(2, -126, 200, 150),
                        anchor: Alignment::LEFT,
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (0.8, 0.8),
                            text_alignment: Alignment::LEFT,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                ],
                ..Pane::EMPTY
            }
            .into(),
            Pane {
                name: "result_pane",
                alpha: 0,
                children: vec![
                    Pane {
                        name: "calculated_text",
                        rect: Rect::new(0, 90, 850, 147),
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: crate::generator::SWITZERLAND_COLOR,
                            scale: (0.6, 0.59),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                    Pane {
                        name: "power_text",
                        rect: Rect::new(5, -106, 800, 294),
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: Color::WHITE,
                            scale: (1.2, 1.19),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                ],
                ..Pane::EMPTY
            }
            .into(),
            Pane {
                name: "rank_pane",
                rect: Rect::new(-12, -93, 45, 60),
                alpha: 0,
                children: vec![
                    Pane {
                        name: "position_text",
                        rect: Rect::new(0, 180, 600, 105),
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: crate::generator::SWITZERLAND_COLOR,
                            scale: (0.6, 0.59),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                    Pane {
                        name: "estimate_text",
                        rect: Rect::new(-170, 47, 300, 93),
                        parent_anchor: Alignment::LEFT,
                        contents: PaneContents::Text {
                            text: "".into(),
                            font: bold_font.clone(),
                            color: Color::RGB(0x80, 0x80, 0x80),
                            scale: (0.6, 0.59),
                            text_alignment: Alignment::CENTER,
                        },
                        ..Pane::EMPTY
                    }
                    .into(),
                    Pane {
                        name: "inner_rank_pane",
                        rect: Rect::new(7, -52, 45, 60),
                        alpha: 0,
                        children: vec![
                            Pane {
                                name: "rank_text",
                                rect: Rect::new(0, 8, 680, 180),
                                parent_anchor: Alignment::LEFT,
                                contents: PaneContents::Text {
                                    text: "".into(),
                                    font: bold_font.clone(),
                                    color: Color::WHITE,
                                    scale: (1.2, 1.19),
                                    text_alignment: Alignment::CENTER,
                                },
                                ..Pane::EMPTY
                            }
                            .into(),
                        ],
                        ..Pane::EMPTY
                    }
                    .into(),
                ],
                ..Pane::EMPTY
            }
            .into(),
        ],
        ..Pane::EMPTY
    }
    .build())
}

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
