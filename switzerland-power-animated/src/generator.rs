use crate::PowerStatus;
use crate::Result;
use crate::alignment::Alignment;
use crate::alignment::HorizontalAlignment::Left;
use crate::alignment::VerticalAlignment::Middle;
use crate::font::FontSet;
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

const WIDTH: u32 = 1015;
const HEIGHT: u32 = 630;
const HALF_WIDTH: i32 = WIDTH as i32 / 2;
const HALF_HEIGHT: i32 = HEIGHT as i32 / 2;
const FPS: u32 = 30;

pub struct AnimationGenerator {
    generator: RefCell<FrameGenerator>,
}

impl AnimationGenerator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            generator: RefCell::new(FrameGenerator::new()?),
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

    fn generate_frames(&self, status: PowerStatus) -> Result<FramesVec> {
        self.generator.borrow_mut().generate_frames(status)
    }
}

type FramesVec = Vec<(Vec<u8>, u32)>;

struct FrameGenerator {
    canvas: SurfaceCanvas<'static>,
    frames: FramesVec,

    background: Surface<'static>,
    swiss_flag: Surface<'static>,
    bold_font: FontSet<'static>,

    _sdl_ttf: Sdl2TtfContext,
    _sdl_image: Sdl2ImageContext,
    _sdl: Sdl,
}

impl FrameGenerator {
    fn new() -> Result<Self> {
        let sdl = sdl2::init()?;
        let sdl_image = sdl2::image::init(InitFlag::PNG)?;
        let sdl_ttf = sdl2::ttf::init()?;

        let canvas = Surface::new(WIDTH, HEIGHT, PixelFormatEnum::ABGR8888)?.into_canvas()?;

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
            canvas,
            frames: vec![],

            background,
            swiss_flag: RWops::from_bytes(include_bytes!("assets/swiss-flag.png"))?.load_png()?,
            bold_font: load_font!(80, "BlitzBold.otf", "FOT-RowdyStd-EB.otf"),

            _sdl: sdl,
            _sdl_image: sdl_image,
            _sdl_ttf: sdl_ttf,
        })
    }

    fn generate_frames(&mut self, status: PowerStatus) -> Result<FramesVec> {
        match status {
            PowerStatus::Calculating { progress, total } => {
                self.generate_calculating(progress, total)?
            }
            _ => todo!("Implement other statuses"),
        }

        Ok(std::mem::take(&mut self.frames))
    }

    fn generate_calculating(&mut self, progress: u32, total: u32) -> Result<()> {
        self.canvas.set_draw_color((0, 0, 0, 0));
        self.canvas.fill_rect(None)?;

        self.background
            .blit_scaled(None, self.canvas.surface_mut(), None)?;

        self.swiss_flag.blit(
            None,
            self.canvas.surface_mut(),
            Rect::new(HALF_WIDTH - 34, HALF_HEIGHT - 190 - 34, 68, 68),
        )?;

        for i in 0..101 {
            let x = HALF_WIDTH - 405 + i * 8;
            const Y: i32 = HALF_HEIGHT + 48 - 5;
            self.canvas
                .filled_circle(x as i16 + 4, Y as i16 + 4, 2, (64, 255, 255, 255))?;
        }

        self.print_text(
            (0, -50 + 90, 850, 147),
            Alignment::CENTER,
            Alignment::CENTER,
            (0.6, 0.59),
            Color::WHITE,
            get_text("en", "calculating"),
        )?;

        self.print_text(
            (-54, -50 - 108, 200, 206),
            Alignment::CENTER,
            Alignment::CENTER,
            (1.2, 1.2),
            Color::WHITE,
            &(progress - 1).to_string(),
        )?;

        self.print_text(
            (2, -50 - 126, 200, 150),
            (Left, Middle),
            (Left, Middle),
            (0.8, 0.8),
            Color::WHITE,
            &format!("/{total}"),
        )?;

        self.push_frame(1);
        Ok(())
    }

    fn print_text(
        &mut self,
        container: impl Into<Rect>,
        anchor: impl Into<Alignment>,
        text_alignment: impl Into<Alignment>,
        (x_scale, y_scale): (f64, f64),
        color: impl Into<Color>,
        text: &str,
    ) -> Result<()> {
        let container = container.into();
        let anchor = anchor.into();
        let text_alignment = text_alignment.into();

        let text = self.bold_font.render(color, text)?;

        let container_x =
            HALF_WIDTH + container.x() - anchor.horizontal.align(container.width() as i32);
        let container_y =
            HALF_HEIGHT - container.y() - anchor.vertical.align(container.height() as i32);
        let text_rel_x = text_alignment.horizontal.align(container.width() as i32)
            - text_alignment
                .horizontal
                .align((text.width() as f64 * x_scale) as i32);
        let text_rel_y = text_alignment.vertical.align(container.height() as i32)
            - text_alignment
                .vertical
                .align((text.height() as f64 * y_scale) as i32);

        let rect = Rect::new(
            container_x + text_rel_x,
            container_y + text_rel_y,
            (text.width() as f64 * x_scale) as u32,
            (text.height() as f64 * y_scale) as u32,
        );
        text.blit_scaled(None, self.canvas.surface_mut(), rect)?;
        Ok(())
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
        const FRAME_MS: u32 = 1000 / FPS;
        const CATCHUP_FRAMES: u32 = (1.0 / (1000.0 / FPS as f64 - FRAME_MS as f64)).round() as u32;
        encoder.add_frame(AnimFrame::from_rgba(
            frame,
            WIDTH,
            HEIGHT,
            (frame_number * FRAME_MS + frame_number / CATCHUP_FRAMES) as i32,
        ));
        frame_number += duration_frames;
    }

    Ok(encoder.try_encode()?)
}
