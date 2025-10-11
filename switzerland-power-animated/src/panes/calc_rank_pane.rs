use crate::Result;
use crate::alignment::Alignment;
use crate::animation::AnimatableParameter::{Alpha, Scale};
use crate::animation::{AnimationSet, AnimationSetElement, AnimationTrack, Keyframe};
use crate::font::FontSet;
use crate::generator::{HEIGHT, SWITZERLAND_COLOR, WIDTH};
use crate::layout::{BuiltPane, ExtraBehavior, Pane, PaneContents, TextPaneContents};
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::rect::Rect;
use sdl2::render::BlendMode;
use std::rc::Rc;

pub fn calc_rank_pane(
    bold_font: Rc<FontSet<'static>>,
    swiss_flag: PaneContents,
) -> Result<BuiltPane> {
    Ok(Pane {
        rect: Rect::new(0, 0, WIDTH, HEIGHT),
        children: vec![
            Pane {
                rect: Rect::new(0, 0, 1015, 630),
                contents: PaneContents::image_png(include_bytes!(
                    "images/calc-rank-background.png"
                ))?,
                contents_blending: BlendMode::None,
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                rect: Rect::new(0, 190, 68, 68),
                contents: swiss_flag,
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                rect: Rect::new(0, 2, 811, 10),
                contents: PaneContents::Custom(|canvas, rect, alpha| {
                    for i in 0..101 {
                        let x = rect.x + i * 8;
                        // WARNING: filled_circle takes in ABGR instead of RGBA
                        canvas.filled_circle(
                            x as i16 + 4,
                            rect.y as i16 + 4,
                            2,
                            (alpha / 4, 255, 255, 255),
                        )?;
                    }
                    Ok(())
                }),
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                name: "progress_pane",
                children: vec![
                    Pane {
                        name: "calculating_text",
                        rect: Rect::new(0, 90, 850, 147),
                        contents: PaneContents::Text(
                            TextPaneContents::new("Calculating", &bold_font).scale(0.6, 0.59),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "progress_text",
                        rect: Rect::new(-54, -108, 200, 206),
                        contents: PaneContents::Text(
                            TextPaneContents::new("3", &bold_font).scale(1.2, 1.19),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "total_text",
                        rect: Rect::new(2, -126, 200, 150),
                        anchor: Alignment::LEFT,
                        contents: PaneContents::Text(
                            TextPaneContents::new("5", &bold_font)
                                .scale(0.8, 0.8)
                                .alignment(Alignment::LEFT),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                ],
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                name: "result_pane",
                alpha: 0,
                children: vec![
                    Pane {
                        name: "calculated_text",
                        rect: Rect::new(0, 90, 850, 147),
                        contents: PaneContents::Text(
                            TextPaneContents::new("Calculated", &bold_font)
                                .color(SWITZERLAND_COLOR)
                                .scale(0.6, 0.59),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "power_value_text",
                        rect: Rect::new(5, -106, 800, 294),
                        contents: PaneContents::Text(
                            TextPaneContents::new("1500.0", &bold_font).scale(1.2, 1.19),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                ],
                ..Pane::EMPTY
            }
            .build(),
            Pane {
                name: "rank_pane",
                rect: Rect::new(-12, -93, 45, 60),
                alpha: 0,
                children: vec![
                    Pane {
                        name: "position_text",
                        rect: Rect::new(0, 180, 600, 105),
                        contents: PaneContents::Text(
                            TextPaneContents::new("Position", &bold_font)
                                .color(SWITZERLAND_COLOR)
                                .scale(0.6, 0.59)
                                .secondary_scale(0.8),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "estimate_text",
                        rect: Rect::new(-170, 47, 300, 93),
                        parent_anchor: Alignment::LEFT,
                        contents: PaneContents::Text(
                            TextPaneContents::new("Estimate", &bold_font)
                                .color((0x80, 0x80, 0x80))
                                .scale(0.6, 0.59)
                                .secondary_scale(0.8),
                        ),
                        ..Pane::EMPTY
                    }
                    .build(),
                    Pane {
                        name: "inner_rank_pane",
                        rect: Rect::new(7, -52, 45, 60),
                        alpha: 0,
                        children: vec![
                            Pane {
                                name: "rank_value_text",
                                rect: Rect::new(0, 8, 680, 180),
                                contents: PaneContents::Text(
                                    TextPaneContents::new("#25", &bold_font)
                                        .scale(1.2, 1.19)
                                        .secondary_scale(0.8),
                                ),
                                ..Pane::EMPTY
                            }
                            .build(),
                            rank_arrows()?,
                        ],
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

fn rank_arrows() -> Result<BuiltPane> {
    fn arrow(name: &'static str, bytes: &'static [u8]) -> Result<BuiltPane> {
        Ok(Pane {
            name,
            rect: Rect::new(0, 0, 165, 165),
            alpha: 0,
            contents: PaneContents::image_png(bytes)?,
            ..Pane::EMPTY
        }
        .build())
    }

    Ok(Pane {
        name: "rank_arrow_root",
        rect: Rect::new(0, 8, 400, 128),
        extra_behavior: ExtraBehavior::AdjustToContentBounds {
            sibling: "rank_value_text",
            min_width: 300,
            margin: 80,
        },
        children: vec![
            Pane {
                name: "inner",
                rect: Rect::new(0, -4, 45, 60),
                parent_anchor: Alignment::RIGHT,
                children: vec![
                    Pane {
                        name: "inner_inner",
                        rect: Rect::new(20, 0, 165, 165),
                        children: vec![
                            arrow(
                                "rank_stay_arrow",
                                include_bytes!("images/rank-stay-arrow.png"),
                            )?,
                            arrow("rank_up_arrow", include_bytes!("images/rank-up-arrow.png"))?,
                            arrow(
                                "rank_down_arrow",
                                include_bytes!("images/rank-down-arrow.png"),
                            )?,
                        ],
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
                Keyframe::new(0.0, -70.0, 1.23238242),
                Keyframe::new(0.0, 5.0, 1.23238242),
                Keyframe::new(-0.000431764929, 5.0, 1.2),
                Keyframe::new(-0.0395297222, 5.0, 1.2),
                Keyframe::new(-0.0395297222, 11.0, 0.9628217),
                Keyframe::new(0.0192887783, 11.0, 0.9628217),
                Keyframe::new(0.00413092, 14.0, 1.020688),
                Keyframe::new(0.0, 20.0, 1.0),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[
                Keyframe::new(0.0, 5.0, 0.0),
                Keyframe::new(0.0, 13.0, 255.0),
            ]),
        ),
    ],
)
.to_set();

pub const PROGRESS_IN: AnimationSet<1> = AnimationSetElement::new(
    &["progress_pane", "progress_text"],
    &[
        (
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.1633873, 0.0, 0.4347826),
                Keyframe::new(0.0, 5.0, 1.25171912),
                Keyframe::new(0.0, 9.0, 1.0),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[Keyframe::new(0.0, 0.0, 0.0), Keyframe::new(0.0, 3.0, 255.0)]),
        ),
    ],
)
.to_set();

pub const RESULT_POWER_IN: AnimationSet<3> = AnimationSet::new([
    AnimationSetElement::new(
        &["progress_pane"],
        &[
            (
                Scale,
                AnimationTrack::new(&[
                    Keyframe::new(-0.0700630844, 0.0, 1.0),
                    Keyframe::new(-0.0005248937, 3.0, 0.905991733),
                ]),
            ),
            (
                Alpha,
                AnimationTrack::new(&[
                    Keyframe::new(-85.0, 0.0, 255.0),
                    Keyframe::new(-85.0, 3.0, 0.0),
                ]),
            ),
        ],
    ),
    AnimationSetElement::new(
        &["result_pane"],
        &[(
            Alpha,
            AnimationTrack::new(&[Keyframe::new(0.0, 2.0, 0.0), Keyframe::new(0.0, 6.0, 255.0)]),
        )],
    ),
    AnimationSetElement::new(
        &["result_pane", "power_value_text"],
        &[(
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.0354311466, 3.0, 1.0),
                Keyframe::new(-0.006858194, 6.0, 1.10629344),
                Keyframe::new(-0.0702922046, 8.0, 0.965709031),
                Keyframe::new(0.03429097, 8.0, 0.965709031),
                Keyframe::new(0.0, 9.0, 1.0),
            ]),
        )],
    ),
]);

pub const RESULT_RANK_IN: AnimationSet<1> = AnimationSetElement::new(
    &["rank_pane", "inner_rank_pane"],
    &[
        (
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.0, 0.0, 2.0),
                Keyframe::new(0.0, 10.0, 0.8074777),
                Keyframe::new(0.0, 13.0, 1.062221),
                Keyframe::new(-0.0311105251, 15.0, 1.0),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[
                Keyframe::new(0.0, 0.0, 0.0),
                Keyframe::new(0.0, 15.0, 255.0),
            ]),
        ),
    ],
)
.to_set();

pub const WINDOW_OUT: AnimationSet<1> = AnimationSetElement::new(
    &[],
    &[
        (
            Scale,
            AnimationTrack::new(&[
                Keyframe::new(0.04875219, 0.0, 1.0),
                Keyframe::new(0.0, 10.0, 1.23238242),
            ]),
        ),
        (
            Alpha,
            AnimationTrack::new(&[
                Keyframe::new(0.0, 0.0, 255.0),
                Keyframe::new(0.0, 10.0, 0.0),
            ]),
        ),
    ],
)
.to_set();
