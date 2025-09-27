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

pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

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

        let canvas = Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?;

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
        let mut base_window = Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?;
        base_window.surface_mut().set_blend_mode(BlendMode::None)?;

        self.background.blit_scaled(
            None,
            base_window.surface_mut(),
            Rect::new(0, 0, 1015, 630).centered_on((HALF_WIDTH, HALF_HEIGHT)),
        )?;

        self.swiss_flag.blit(
            None,
            base_window.surface_mut(),
            Rect::new(HALF_WIDTH - 34, HALF_HEIGHT - 190 - 34, 68, 68),
        )?;

        for i in 0..101 {
            let x = HALF_WIDTH - 405 + i * 8;
            const Y: i32 = HALF_HEIGHT + 48 - 5;
            // WARNING: filled_circle takes in ABGR instead of RGBA
            base_window.filled_circle(x as i16 + 4, Y as i16 + 4, 2, (64, 255, 255, 255))?;
        }

        self.print_text(
            &mut base_window,
            (0, -50 + 90, 850, 147),
            Alignment::CENTER,
            Alignment::CENTER,
            (0.6, 0.59),
            get_text("en", "calculating"),
        )?;

        self.print_text(
            &mut base_window,
            (2, -50 - 126, 200, 150),
            (Left, Middle),
            (Left, Middle),
            (0.8, 0.8),
            &format!("/{total}"),
        )?;

        {
            let mut base_window_with_progress =
                Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?;
            base_window_with_progress.set_blend_mode(BlendMode::Mul);
            base_window
                .surface()
                .blit(None, base_window_with_progress.surface_mut(), None)?;
            self.print_text(
                &mut base_window_with_progress,
                (-54, -50 - 108, 200, 206),
                Alignment::CENTER,
                Alignment::CENTER,
                (1.2, 1.2),
                &(progress - 1).to_string(),
            )?;

            self.canvas.set_draw_color((0, 0, 0, 0));
            for frame in 0..=WINDOW_IN_SCALE.duration() as u32 {
                let scale = WINDOW_IN_SCALE.value_at(frame as f64);
                let alpha = WINDOW_IN_ALPHA.value_at(frame as f64);
                self.canvas.fill_rect(None)?;
                base_window_with_progress
                    .surface_mut()
                    .set_alpha_mod(alpha as u8);
                base_window_with_progress.surface().blit_scaled(
                    None,
                    self.canvas.surface_mut(),
                    Rect::new(
                        HALF_WIDTH - (HALF_WIDTH as f64 * scale) as i32,
                        HALF_HEIGHT - (HALF_HEIGHT as f64 * scale) as i32,
                        (WIDTH as f64 * scale) as u32,
                        (HEIGHT as f64 * scale) as u32,
                    ),
                )?;
                self.push_frame(1);
            }

            base_window_with_progress.surface_mut().set_alpha_mod(255);
            base_window_with_progress
                .surface()
                .blit(None, self.canvas.surface_mut(), None)?;
            self.push_frame(60);
        }

        base_window
            .surface()
            .blit(None, self.canvas.surface_mut(), None)?;
        self.push_frame(1);

        {
            let mut progress_text = self.bold_font.render(Color::WHITE, &progress.to_string())?;
            fn layout_progress_text(
                text: &Surface,
                canvas: &mut SurfaceCanvas,
                scale: f64,
            ) -> Result<()> {
                FrameGenerator::layout_surface(
                    canvas,
                    (-54, -50 - 108, 200, 206),
                    Alignment::CENTER,
                    Alignment::CENTER,
                    (1.2 * scale, 1.2 * scale),
                    text,
                )
            }

            for frame in 0..=PROGRESS_IN_SCALE.duration() as u32 {
                let scale = PROGRESS_IN_SCALE.value_at(frame as f64);
                let alpha = PROGRESS_IN_ALPHA.value_at(frame as f64);
                base_window
                    .surface()
                    .blit(None, self.canvas.surface_mut(), None)?;
                progress_text.set_alpha_mod(alpha as u8);
                layout_progress_text(&progress_text, &mut self.canvas, scale)?;
                self.push_frame(1);
            }

            layout_progress_text(&progress_text, &mut base_window, 1.0)?;
        }

        base_window
            .surface()
            .blit(None, self.canvas.surface_mut(), None)?;
        self.push_frame(60);

        {
            self.canvas.set_draw_color((0, 0, 0, 0));
            for frame in 0..=WINDOW_OUT_SCALE.duration() as u32 {
                let scale = WINDOW_OUT_SCALE.value_at(frame as f64);
                let alpha = WINDOW_OUT_ALPHA.value_at(frame as f64);
                self.canvas.fill_rect(None)?;
                base_window.surface_mut().set_alpha_mod(alpha as u8);
                base_window.surface().blit_scaled(
                    None,
                    self.canvas.surface_mut(),
                    Rect::new(
                        HALF_WIDTH - (HALF_WIDTH as f64 * scale) as i32,
                        HALF_HEIGHT - (HALF_HEIGHT as f64 * scale) as i32,
                        (WIDTH as f64 * scale) as u32,
                        (HEIGHT as f64 * scale) as u32,
                    ),
                )?;
                self.push_frame(1);
            }

            self.canvas.fill_rect(None)?;
            self.push_frame(60);
            self.push_frame(0);
        }

        Ok(())
    }

    fn print_text(
        &self,
        canvas: &mut SurfaceCanvas,
        container: impl Into<Rect>,
        anchor: impl Into<Alignment>,
        text_alignment: impl Into<Alignment>,
        (x_scale, y_scale): (f64, f64),
        text: &str,
    ) -> Result<()> {
        Self::layout_surface(
            canvas,
            container,
            anchor,
            text_alignment,
            (x_scale, y_scale),
            &self.bold_font.render(Color::WHITE, text)?,
        )
    }

    fn layout_surface(
        canvas: &mut SurfaceCanvas,
        container: impl Into<Rect>,
        anchor: impl Into<Alignment>,
        surface_alignment: impl Into<Alignment>,
        (x_scale, y_scale): (f64, f64),
        surface: &Surface,
    ) -> Result<()> {
        let container = container.into();
        let anchor = anchor.into();
        let surface_alignment = surface_alignment.into();

        let container_x =
            HALF_WIDTH + container.x() - anchor.horizontal.align(container.width() as i32);
        let container_y =
            HALF_HEIGHT - container.y() - anchor.vertical.align(container.height() as i32);
        let text_rel_x = surface_alignment.horizontal.align(container.width() as i32)
            - surface_alignment
                .horizontal
                .align((surface.width() as f64 * x_scale) as i32);
        let text_rel_y = surface_alignment.vertical.align(container.height() as i32)
            - surface_alignment
                .vertical
                .align((surface.height() as f64 * y_scale) as i32);

        let rect = Rect::new(
            container_x + text_rel_x,
            container_y + text_rel_y,
            (surface.width() as f64 * x_scale) as u32,
            (surface.height() as f64 * y_scale) as u32,
        );
        surface.blit_scaled(None, canvas.surface_mut(), rect)?;
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
