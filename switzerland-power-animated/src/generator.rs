use crate::PowerStatus;
use crate::Result;
use crate::font::FontSet;
use sdl2::Sdl;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::SurfaceCanvas;
use sdl2::surface::Surface;
use sdl2::ttf::Sdl2TtfContext;
use std::cell::RefCell;
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

const WIDTH: u32 = 1350;
const HEIGHT: u32 = 800;
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
    bold_font: FontSet<'static>,
    frames: FramesVec,

    _sdl_ttf: Sdl2TtfContext,
    _sdl: Sdl,
}

impl FrameGenerator {
    fn new() -> Result<Self> {
        let sdl = sdl2::init()?;
        let canvas = Surface::new(WIDTH, HEIGHT, PixelFormatEnum::RGBA8888)?.into_canvas()?;

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

        Ok(Self {
            canvas,
            bold_font: load_font!(80, "BlitzBold.otf", "FOT-RowdyStd-EB.otf"),
            frames: vec![],

            _sdl: sdl,
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

        let text = self
            .bold_font
            .render(Color::WHITE, &(progress - 1).to_string())?;
        text.blit_scaled(
            None,
            self.canvas.surface_mut(),
            Rect::new(
                WIDTH as i32 / 2 - 54 - (text.width() as f64 * 0.6) as i32,
                HEIGHT as i32 / 2 + 108 - (text.height() as f64 * 0.6) as i32,
                (text.width() as f64 * 1.2) as u32,
                (text.height() as f64 * 1.2) as u32,
            ),
        )?;
        drop(text);

        let text = self.bold_font.render(Color::WHITE, &format!("/{total}"))?;
        text.blit_scaled(
            None,
            self.canvas.surface_mut(),
            Rect::new(
                WIDTH as i32 / 2 + 2 - (text.width() as f64 * 0.4) as i32,
                HEIGHT as i32 / 2 + 126 - (text.height() as f64 * 0.4) as i32,
                (text.width() as f64 * 0.8) as u32,
                (text.height() as f64 * 0.8) as u32,
            ),
        )?;
        drop(text);

        self.push_frame(1);
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
