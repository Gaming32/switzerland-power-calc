use crate::font::FontPair;
use crate::PowerStatus;
use crate::Result;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::SurfaceCanvas;
use sdl2::surface::Surface;
use sdl2::ttf::Sdl2TtfContext;
use sdl2::Sdl;
use std::cell::RefCell;
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

const WIDTH: u32 = 1350;
const HEIGHT: u32 = 800;
const FPS: i32 = 30;

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

    fn generate_frames(&self, status: PowerStatus) -> Result<Vec<Vec<u8>>> {
        self.generator.borrow_mut().generate_frames(status)
    }
}

struct FrameGenerator {
    canvas: SurfaceCanvas<'static>,
    _sdl: Sdl,

    bold_font: FontPair<'static>,
    sdl_ttf: Sdl2TtfContext,

    frames: Vec<Vec<u8>>,
}

impl FrameGenerator {
    fn new() -> Result<Self> {
        let sdl = sdl2::init()?;
        let canvas = Surface::new(WIDTH, HEIGHT, PixelFormatEnum::RGBA8888)?.into_canvas()?;

        let sdl_ttf = sdl2::ttf::init()?;
        macro_rules! load_font {
            ($main:literal, $fallback:literal, $point_size:literal) => {
                FontPair::load(
                    unsafe { core::mem::transmute::<&_, &'static _>(&sdl_ttf) },
                    include_bytes!(concat!("assets/", $main)),
                    include_bytes!(concat!("assets/", $fallback)),
                    $point_size,
                )?
            };
        }

        Ok(Self {
            canvas,
            _sdl: sdl,

            bold_font: load_font!("BlitzBold.otf", "FOT-RowdyStd-EB.otf", 80),
            sdl_ttf,

            frames: vec![],
        })
    }

    fn generate_frames(&mut self, status: PowerStatus) -> Result<Vec<Vec<u8>>> {
        match status {
            PowerStatus::Calculating { progress, total } => self.generate_calculating(progress, total)?,
            _ => todo!("Implement other statuses"),
        }

        Ok(std::mem::take(&mut self.frames))
    }

    fn generate_calculating(&mut self, progress: u32, total: u32) -> Result<()> {
        self.canvas.set_draw_color((0, 0, 0, 0));
        self.canvas.fill_rect(None)?;

        let text = self.bold_font.main.render_latin1(progress.to_string().as_bytes())
            .blended(Color::WHITE)
            .unwrap();
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

        let text = self.bold_font.main.render_latin1(format!("/{total}").as_bytes())
            .blended(Color::WHITE)
            .unwrap();
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

        self.push_frame();
        Ok(())
    }

    fn push_frame(&mut self) {
        self.frames.push(self.canvas.surface().without_lock().unwrap().into());
    }
}

fn encode_frames(frames: Vec<Vec<u8>>) -> Result<WebPMemory> {
    let webp_config = WebPConfig::new().unwrap();
    let mut encoder = AnimEncoder::new(WIDTH, HEIGHT, &webp_config);

    for (i, frame) in frames.iter().enumerate() {
        encoder.add_frame(AnimFrame::from_rgba(
            frame,
            WIDTH,
            HEIGHT,
            i as i32 * 1000 / FPS,
        ));
    }

    Ok(encoder.try_encode()?)
}
