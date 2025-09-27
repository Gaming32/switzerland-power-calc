use std::borrow::Cow;
use crate::PowerStatus;
use crate::Result;
use crate::alignment::Alignment;
use crate::alignment::HorizontalAlignment::Left;
use crate::alignment::VerticalAlignment::Middle;
use crate::animations::{
    PROGRESS_IN_ALPHA, PROGRESS_IN_SCALE, WINDOW_IN_ALPHA, WINDOW_IN_SCALE, WINDOW_OUT_ALPHA,
    WINDOW_OUT_SCALE,
};
use crate::font::FontSet;
use crate::layout::{Pane, PaneContents, BuiltPane};
use crate::texts::get_text;
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
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

const WIDTH: u32 = 1250;
const HEIGHT: u32 = 776;
const HALF_WIDTH: i32 = WIDTH as i32 / 2;
const HALF_HEIGHT: i32 = HEIGHT as i32 / 2;
const FPS: u32 = 60;
const LANG: &str = "en"; // TODO: Make configurable

pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

pub struct AnimationGenerator {
    state: RefCell<GeneratorState>,

    background: Surface<'static>,
    swiss_flag: Surface<'static>,
    bold_font: FontSet<'static>,

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

        let mut background =
            RWops::from_bytes(include_bytes!("assets/background.png"))?.load_png()?;
        background.set_blend_mode(BlendMode::None)?;

        Ok(Self {
            state: RefCell::new(GeneratorState {
                canvas: Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?,
                frames: vec![],
            }),

            background,
            swiss_flag: RWops::from_bytes(include_bytes!("assets/swiss-flag.png"))?.load_png()?,
            bold_font: load_font!(80, "BlitzBold.otf", "FOT-RowdyStd-EB.otf"),

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
                self.generate_calculating(progress, total, None)?
            }
            PowerStatus::Calculated {
                calculation_rounds,
                power,
                rank: _,
            } => self.generate_calculating(calculation_rounds, calculation_rounds, Some(power))?,
            _ => todo!("Implement other statuses"),
        }

        Ok(std::mem::take(&mut self.state.borrow_mut().frames))
    }

    fn generate_calculating(
        &self,
        progress: u32,
        total: u32,
        _calculated_power: Option<f64>,
    ) -> Result<()> {
        let mut state = self.state.borrow_mut();

        let root_pane = Pane {
            rect: state.canvas.surface().rect(),
            children: vec![
                Pane {
                    rect: Rect::new(0, 0, 1015, 630),
                    contents: PaneContents::Image(&self.background),
                    ..Default::default()
                }
                .into(),
                Pane {
                    rect: Rect::new(0, 190, 68, 68),
                    contents: PaneContents::Image(&self.swiss_flag),
                    ..Default::default()
                }
                .into(),
                Pane {
                    rect: Rect::new(0, 0, 811, 10),
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
                            rect: Rect::new(0, 90, 850, 147),
                            contents: PaneContents::Text {
                                text: get_text(LANG, "calculating").into(),
                                font: &self.bold_font,
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
                                text: (progress - 1).to_string().into(),
                                font: &self.bold_font,
                                color: Color::WHITE,
                                scale: (1.2, 1.19),
                                text_alignment: Alignment::CENTER,
                            },
                            ..Default::default()
                        }
                        .into(),
                        Pane {
                            rect: Rect::new(2, -126, 200, 150),
                            anchor: Alignment::LEFT,
                            contents: PaneContents::Text {
                                text: format!("/{total}").into(),
                                font: &self.bold_font,
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
            ],
            ..Default::default()
        }.build();
        let progress_text = root_pane
            .child(&["progress_pane", "progress_text"])
            .unwrap();

        for frame in 0..=WINDOW_IN_SCALE.duration() as u32 {
            root_pane.edit(|x| {
                x.scale = WINDOW_IN_SCALE.value_at(frame as f64);
                x.alpha = WINDOW_IN_ALPHA.value_at(frame as f64) as u8;
            });
            state.render_frame(&root_pane, 1)?;
        }

        root_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
        });
        state.render_frame(&root_pane, 60)?;

        progress_text.edit(|x| x.alpha = 0);
        state.render_frame(&root_pane, 1)?;

        progress_text.edit(|x| {
            if let PaneContents::Text { text, .. } = &mut x.contents {
                *text = Cow::Owned(progress.to_string());
            }
        });
        for frame in 0..=PROGRESS_IN_SCALE.duration() as u32 {
            progress_text.edit(|x| {
                x.scale = PROGRESS_IN_SCALE.value_at(frame as f64);
                x.alpha = PROGRESS_IN_ALPHA.value_at(frame as f64) as u8;
            });
            state.render_frame(&root_pane, 1)?;
        }

        progress_text.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
        });
        state.render_frame(&root_pane, 60)?;

        for frame in 0..=WINDOW_OUT_SCALE.duration() as u32 {
            root_pane.edit(|x| {
                x.scale = WINDOW_OUT_SCALE.value_at(frame as f64);
                x.alpha = WINDOW_OUT_ALPHA.value_at(frame as f64) as u8;
            });
            state.render_frame(&root_pane, 1)?;
        }

        root_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 0;
        });
        state.render_frame(&root_pane, 60)?;

        state.push_frame(0);

        Ok(())
    }
}

type FramesVec = Vec<(Vec<u8>, u32)>;

struct GeneratorState {
    canvas: SurfaceCanvas<'static>,
    frames: FramesVec,
}

impl GeneratorState {
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
