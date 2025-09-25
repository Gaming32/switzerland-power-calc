use crate::error::Result;
use sdl2::Sdl;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::surface::Surface;
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const FPS: i32 = 30;

pub struct AnimationGenerator {
    _sdl: Sdl,
}

impl AnimationGenerator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            _sdl: sdl2::init()?,
        })
    }

    pub fn generate(&self) -> Result<WebPMemory> {
        encode_frames(generate_frames()?)
    }

    pub async fn generate_async(&self) -> Result<Vec<u8>> {
        let frames = generate_frames()?;
        tokio::task::spawn_blocking(move || encode_frames(frames).map(|x| x.to_vec()))
            .await
            .unwrap()
    }
}

fn generate_frames() -> Result<Vec<Vec<u8>>> {
    println!("Generate thread: {:?}", std::thread::current().id());

    let mut canvas = Surface::new(WIDTH, HEIGHT, PixelFormatEnum::RGBA8888)?.into_canvas()?;
    let mut frames = vec![];
    for i in (0..HEIGHT).step_by(2) {
        canvas.set_draw_color((0, 0, 0, 0));
        canvas.fill_rect(None)?;
        canvas.filled_circle((WIDTH / 2) as i16, i as i16, 100, Color::RED)?;
        frames.push(canvas.surface().without_lock().unwrap().into());
    }

    Ok(frames)
}

fn encode_frames(frames: Vec<Vec<u8>>) -> Result<WebPMemory> {
    println!("Encode thread: {:?}", std::thread::current().id());

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
