use crate::Result;
use crate::font::FontSet;
use crate::generator::{HEIGHT, WIDTH};
use crate::layout::{BuiltPane, Pane, PaneContents};
use sdl2::rect::Rect;
use std::rc::Rc;
use sdl2::pixels::Color;
use crate::alignment::Alignment;

pub fn power_progress_pane(font: Rc<FontSet<'static>>) -> Result<BuiltPane> {
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
            .into(),
            Pane {
                rect: Rect::new(0, 264, 68, 68),
                contents: PaneContents::image_png(include_bytes!("images/swiss-flag.png"))?,
                ..Pane::EMPTY
            }
            .into(),
            Pane {
                name: "set_outcome_pane",
                rect: Rect::new(0, -79, 30, 40),
                children: vec![
                    Pane {
                        name: "set_score_text",
                        rect: Rect::new(0, 191, 300, 300),
                        alpha: 0,
                        anchor: Alignment::BOTTOM,
                        contents: PaneContents::Text {
                            text: "3-2".into(),
                            font: font.clone(),
                            color: Color::WHITE,
                            scale: (0.7, 0.7),
                            text_alignment: Alignment::BOTTOM,
                        },
                        ..Pane::EMPTY
                    }.into(),
                ],
                ..Pane::EMPTY
            }.into(),
        ],
        ..Pane::EMPTY
    }
    .build())
}

fn win_lose_panel() {
}
