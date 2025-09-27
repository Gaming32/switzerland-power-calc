use crate::PowerStatus;
use crate::Result;
use crate::alignment::Alignment;
use crate::animation::AnimationTrack;
use crate::animations::{
    PROGRESS_IN_ALPHA, PROGRESS_IN_SCALE, RESULT_PANE_ALPHA, RESULT_POWER_SCALE,
    RESULT_PROGRESS_ALPHA, RESULT_PROGRESS_SCALE, RESULT_RANK_ALPHA, RESULT_RANK_SCALE,
    WINDOW_IN_ALPHA, WINDOW_IN_SCALE, WINDOW_OUT_ALPHA, WINDOW_OUT_SCALE,
};
use crate::font::FontSet;
use crate::layout::{BuiltPane, Pane, PaneContents};
use crate::texts::{get_text, get_text_fmt};
use sdl2::Sdl;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::image::{ImageRWops, InitFlag, Sdl2ImageContext};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, SurfaceCanvas};
use sdl2::rwops::RWops;
use sdl2::surface::Surface;
use sdl2::ttf::Sdl2TtfContext;
use std::cell::RefCell;
use std::rc::Rc;
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

const WIDTH: u32 = 1250;
const HEIGHT: u32 = 776;
const FPS: u32 = 60;
const LANG: &str = "en"; // TODO: Make configurable
const SWITZERLAND_COLOR: Color = Color::RGB(218, 41, 28);

pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

pub struct AnimationGenerator {
    state: RefCell<GeneratorState>,

    root_pane: BuiltPane,

    progress_pane: BuiltPane,
    calculating_text: BuiltPane,
    progress_text: BuiltPane,
    total_text: BuiltPane,

    result_pane: BuiltPane,
    calculated_text: BuiltPane,
    power_text: BuiltPane,

    rank_pane: BuiltPane,
    position_text: BuiltPane,
    estimate_text: BuiltPane,
    inner_rank_pane: BuiltPane,
    rank_text: BuiltPane,

    _sdl_ttf: Sdl2TtfContext,
    _sdl_image: Sdl2ImageContext,
    _sdl: Sdl,
}

impl AnimationGenerator {
    pub fn new() -> Result<Self> {
        let sdl = sdl2::init()?;
        let sdl_image = sdl2::image::init(InitFlag::PNG)?;
        let sdl_ttf = sdl2::ttf::init()?;

        macro_rules! load_font {
            ($point_size:literal, $($font:literal),+ $(,)?) => {
                FontSet::load(
                    unsafe { core::mem::transmute::<&_, &'static _>(&sdl_ttf) },
                    $point_size,
                    &[$(
                        include_bytes!(concat!("assets/", $font))
                    ),+],
                )?
            };
        }

        let bold_font = Rc::new(load_font!(80, "BlitzBold.otf", "FOT-RowdyStd-EB.otf"));

        let root_pane = Pane {
            rect: Rect::new(0, 0, WIDTH, HEIGHT),
            children: vec![
                Pane {
                    rect: Rect::new(0, 0, 1015, 630),
                    contents: PaneContents::Image(
                        RWops::from_bytes(include_bytes!("assets/calc-rank-background.png"))?.load_png()?,
                    ),
                    ..Default::default()
                }
                .into(),
                Pane {
                    rect: Rect::new(0, 190, 68, 68),
                    contents: PaneContents::Image(
                        RWops::from_bytes(include_bytes!("assets/swiss-flag.png"))?.load_png()?,
                    ),
                    ..Default::default()
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
                    ..Default::default()
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
                            ..Default::default()
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
                            ..Default::default()
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
                            ..Default::default()
                        }
                        .into(),
                    ],
                    ..Default::default()
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
                                color: SWITZERLAND_COLOR,
                                scale: (0.6, 0.59),
                                text_alignment: Alignment::CENTER,
                            },
                            ..Default::default()
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
                            ..Default::default()
                        }
                        .into(),
                    ],
                    ..Default::default()
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
                                color: SWITZERLAND_COLOR,
                                scale: (0.6, 0.59),
                                text_alignment: Alignment::CENTER,
                            },
                            ..Default::default()
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
                            ..Default::default()
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
                                    ..Default::default()
                                }
                                .into(),
                            ],
                            ..Default::default()
                        }
                        .into(),
                    ],
                    ..Default::default()
                }
                .into(),
            ],
            ..Default::default()
        }
        .build();

        let progress_pane = root_pane.child(&["progress_pane"]).unwrap();
        let calculating_text = progress_pane.child(&["calculating_text"]).unwrap();
        let progress_text = progress_pane.child(&["progress_text"]).unwrap();
        let total_text = progress_pane.child(&["total_text"]).unwrap();

        let result_pane = root_pane.child(&["result_pane"]).unwrap();
        let calculated_text = result_pane.child(&["calculated_text"]).unwrap();
        let power_text = result_pane.child(&["power_text"]).unwrap();

        let rank_pane = root_pane.child(&["rank_pane"]).unwrap();
        let position_text = rank_pane.child(&["position_text"]).unwrap();
        let estimate_text = rank_pane.child(&["estimate_text"]).unwrap();
        let inner_rank_pane = rank_pane.child(&["inner_rank_pane"]).unwrap();
        let rank_text = inner_rank_pane.child(&["rank_text"]).unwrap();

        Ok(Self {
            state: RefCell::new(GeneratorState {
                canvas: Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?,
                frames: vec![],
            }),

            root_pane,

            progress_pane,
            calculating_text,
            progress_text,
            total_text,

            result_pane,
            calculated_text,
            power_text,

            rank_pane,
            position_text,
            estimate_text,
            inner_rank_pane,
            rank_text,

            _sdl: sdl,
            _sdl_image: sdl_image,
            _sdl_ttf: sdl_ttf,
        })
    }

    pub fn generate(&self, status: PowerStatus) -> Result<WebPMemory> {
        encode_frames(self.generate_frames(status)?)
    }

    pub async fn generate_async(&self, status: PowerStatus) -> Result<Vec<u8>> {
        let frames = self.generate_frames(status)?;
        tokio::task::spawn_blocking(move || encode_frames(frames).map(|x| x.to_vec()))
            .await
            .unwrap()
    }
}

impl AnimationGenerator {
    fn generate_frames(&self, status: PowerStatus) -> Result<FramesVec> {
        match status {
            PowerStatus::Calculating { progress, total } => {
                self.generate_calculating(progress, total, None, None)?
            }
            PowerStatus::Calculated {
                calculation_rounds,
                power,
                rank,
            } => self.generate_calculating(
                calculation_rounds,
                calculation_rounds,
                Some(power),
                Some(rank),
            )?,
            _ => todo!("Implement other statuses"),
        }
        self.reset_ui();

        Ok(std::mem::take(&mut self.state.borrow_mut().frames))
    }

    fn generate_calculating(
        &self,
        progress: u32,
        total: u32,
        calculated_power: Option<f64>,
        estimated_rank: Option<u32>,
    ) -> Result<()> {
        let mut state = self.state.borrow_mut();

        self.calculating_text
            .set_text(get_text(LANG, "calculating"));
        self.progress_text.set_text((progress - 1).to_string());
        self.total_text.set_text(format!("/{total}"));

        state.animate_transition(
            &self.root_pane,
            &self.root_pane,
            WINDOW_IN_SCALE,
            WINDOW_IN_ALPHA,
            60,
        )?;

        self.progress_text.edit(|x| x.alpha = 0);
        state.render_frame(&self.root_pane, 1)?;

        self.progress_text.set_text(progress.to_string());
        state.animate_transition(
            &self.root_pane,
            &self.progress_text,
            PROGRESS_IN_SCALE,
            PROGRESS_IN_ALPHA,
            120,
        )?;

        if let Some(calculated_power) = calculated_power {
            self.calculated_text.set_text(get_text(LANG, "calculated"));
            self.power_text.set_text(format!("{:.1}", calculated_power));

            for frame in 0..=RESULT_POWER_SCALE.duration() as u32 {
                self.progress_pane.edit(|x| {
                    x.scale = RESULT_PROGRESS_SCALE.value_at(frame as f64);
                    x.alpha = RESULT_PROGRESS_ALPHA.value_at(frame as f64) as u8;
                });
                self.result_pane.edit(|x| {
                    x.alpha = RESULT_PANE_ALPHA.value_at(frame as f64) as u8;
                });
                self.power_text.edit(|x| {
                    x.scale = RESULT_POWER_SCALE.value_at(frame as f64);
                });
                state.render_frame(&self.root_pane, 1)?;
            }

            self.progress_pane.edit(|x| {
                x.scale = 1.0;
                x.alpha = 0;
            });
            self.result_pane.edit(|x| {
                x.alpha = 255;
            });
            self.power_text.edit(|x| {
                x.scale = 1.0;
            });
            state.render_frame(&self.root_pane, 180)?;
        }

        if let Some(estimated_rank) = estimated_rank {
            state.animate_transition(
                &self.root_pane,
                &self.root_pane,
                WINDOW_OUT_SCALE,
                WINDOW_OUT_ALPHA,
                6,
            )?;

            self.result_pane.edit(|x| x.alpha = 0);
            self.rank_pane.edit(|x| x.alpha = 255);

            self.position_text.set_text(get_text(LANG, "position"));
            self.estimate_text.set_text(get_text(LANG, "estimate"));
            self.rank_text.set_text(get_text_fmt(
                LANG,
                "rank_value",
                vec![("rank", estimated_rank.to_string())],
            ));

            state.animate_transition(
                &self.root_pane,
                &self.root_pane,
                WINDOW_IN_SCALE,
                WINDOW_IN_ALPHA,
                6,
            )?;

            state.animate_transition(
                &self.root_pane,
                &self.inner_rank_pane,
                RESULT_RANK_SCALE,
                RESULT_RANK_ALPHA,
                120,
            )?;
        }

        state.animate_transition(
            &self.root_pane,
            &self.root_pane,
            WINDOW_OUT_SCALE,
            WINDOW_OUT_ALPHA,
            90,
        )?;
        state.push_frame(0);

        Ok(())
    }

    fn reset_ui(&self) {
        self.root_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
        });

        self.progress_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
        });
        self.progress_text.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
            x.set_text("");
        });
        self.total_text.set_text("");

        self.result_pane.edit(|x| {
            x.alpha = 0;
        });
        self.power_text.edit(|x| {
            x.scale = 1.0;
        });

        self.rank_pane.edit(|x| {
            x.alpha = 0;
        });
        self.inner_rank_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 0;
        });
        self.rank_text.set_text("");
    }
}

type FramesVec = Vec<(Vec<u8>, u32)>;

struct GeneratorState {
    canvas: SurfaceCanvas<'static>,
    frames: FramesVec,
}

impl GeneratorState {
    fn animate_transition(
        &mut self,
        root_pane: &BuiltPane,
        pane: &BuiltPane,
        scale_anim: AnimationTrack,
        alpha_anim: AnimationTrack,
        end_delay: u32,
    ) -> Result<()> {
        for frame in 0..=scale_anim.duration() as u32 {
            pane.edit(|x| {
                x.scale = scale_anim.value_at(frame as f64);
                x.alpha = alpha_anim.value_at(frame as f64) as u8;
            });
            self.render_frame(root_pane, 1)?;
        }

        pane.edit(|x| {
            x.scale = scale_anim.ending_value();
            x.alpha = alpha_anim.ending_value() as u8;
        });
        self.render_frame(root_pane, end_delay)?;

        Ok(())
    }

    fn render_frame(&mut self, pane: &BuiltPane, duration_frames: u32) -> Result<()> {
        self.clear_canvas();
        pane.render(&mut self.canvas)?;
        self.push_frame(duration_frames);
        Ok(())
    }

    fn clear_canvas(&mut self) {
        const CLEAR_COLOR: Color = Color::RGBA(0, 0, 0, 0);
        self.canvas.set_draw_color(CLEAR_COLOR);
        self.canvas.set_blend_mode(BlendMode::None);
        self.canvas.clear();
    }

    fn push_frame(&mut self, duration_frames: u32) {
        self.frames.push((
            self.canvas.surface().without_lock().unwrap().into(),
            duration_frames,
        ));
    }
}

fn encode_frames(frames: FramesVec) -> Result<WebPMemory> {
    let webp_config = WebPConfig::new().unwrap();
    let mut encoder = AnimEncoder::new(WIDTH, HEIGHT, &webp_config);

    let mut frame_number = 0;
    for (frame, duration_frames) in frames.iter() {
        const FRAME_TS: u32 = 3000 / FPS;
        encoder.add_frame(AnimFrame::from_rgba(
            frame,
            WIDTH,
            HEIGHT,
            (frame_number * FRAME_TS / 3) as i32,
        ));
        frame_number += duration_frames;
    }

    Ok(encoder.try_encode()?)
}
