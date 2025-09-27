use crate::{MatchOutcome, PowerStatus};
use crate::Result;
use crate::animation::AnimationTrack;
use crate::font::FontSet;
use crate::layout::BuiltPane;
use crate::panes::{calc_rank_pane, power_progress_pane};
use crate::texts::{FmtKey, get_text, get_text_fmt, format_power};
use sdl2::Sdl;
use sdl2::image::{InitFlag, Sdl2ImageContext};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{BlendMode, SurfaceCanvas};
use sdl2::surface::Surface;
use sdl2::ttf::Sdl2TtfContext;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

pub const WIDTH: u32 = 1250;
pub const HEIGHT: u32 = 776;
pub const SWITZERLAND_COLOR: Color = Color::RGB(218, 41, 28);

const FPS: u32 = 60;
const LANG: &str = "USen"; // TODO: Make configurable

pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

pub struct AnimationGenerator {
    state: RefCell<GeneratorState>,

    calc_rank_pane: BuiltPane,
    change_power_pane: BuiltPane,

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
                        include_bytes!(concat!("fonts/", $font))
                    ),+],
                )?
            };
        }

        let bold_font = Rc::new(load_font!(80, "BlitzBold.otf", "FOT-RowdyStd-EB.otf"));

        Ok(Self {
            state: RefCell::new(GeneratorState {
                canvas: Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?,
                frames: vec![],
            }),

            calc_rank_pane: calc_rank_pane::calc_rank_pane(bold_font.clone())?,
            change_power_pane: power_progress_pane::power_progress_pane(bold_font.clone())?,

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
            PowerStatus::SetPlayed { matches, old_power, new_power, old_rank, new_rank } => {
                self.generate_set_played(matches, old_power, new_power, old_rank, new_rank)?;
            },
        }

        Ok(mem::take(&mut self.state.borrow_mut().frames))
    }

    fn generate_calculating(
        &self,
        progress: u32,
        total: u32,
        calculated_power: Option<f64>,
        estimated_rank: Option<u32>,
    ) -> Result<()> {
        use calc_rank_pane::*;

        let mut state = self.state.borrow_mut();

        let progress_pane = self.calc_rank_pane.child(&["progress_pane"]).unwrap();
        let calculating_text = progress_pane.child(&["calculating_text"]).unwrap();
        let progress_text = progress_pane.child(&["progress_text"]).unwrap();
        let total_text = progress_pane.child(&["total_text"]).unwrap();

        let result_pane = self.calc_rank_pane.child(&["result_pane"]).unwrap();
        let calculated_text = result_pane.child(&["calculated_text"]).unwrap();
        let power_text = result_pane.child(&["power_text"]).unwrap();

        let rank_pane = self.calc_rank_pane.child(&["rank_pane"]).unwrap();
        let position_text = rank_pane.child(&["position_text"]).unwrap();
        let estimate_text = rank_pane.child(&["estimate_text"]).unwrap();
        let inner_rank_pane = rank_pane.child(&["inner_rank_pane"]).unwrap();
        let rank_text = inner_rank_pane.child(&["rank_text"]).unwrap();

        self.calc_rank_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
        });

        calculating_text.set_text(get_text(LANG, "calculating"));
        progress_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
        });
        progress_text.edit(|x| {
            x.scale = 1.0;
            x.alpha = 255;
            x.set_text((progress - 1).to_string());
        });
        total_text.set_text(format!("/{total}"));

        result_pane.edit(|x| {
            x.alpha = 0;
        });
        power_text.edit(|x| {
            x.scale = 1.0;
        });

        rank_pane.edit(|x| {
            x.alpha = 0;
        });
        inner_rank_pane.edit(|x| {
            x.scale = 1.0;
            x.alpha = 0;
        });

        state.animate_transition(
            &self.calc_rank_pane,
            &self.calc_rank_pane,
            WINDOW_IN_SCALE,
            WINDOW_IN_ALPHA,
            60,
        )?;

        progress_text.edit(|x| x.alpha = 0);
        state.render_frame(&self.calc_rank_pane, 1)?;

        progress_text.set_text(progress.to_string());
        state.animate_transition(
            &self.calc_rank_pane,
            &progress_text,
            PROGRESS_IN_SCALE,
            PROGRESS_IN_ALPHA,
            120,
        )?;

        if let Some(calculated_power) = calculated_power {
            calculated_text.set_text(get_text(LANG, "calculated"));
            power_text.set_text(format_power(LANG, calculated_power));

            for frame in 0..=RESULT_POWER_SCALE.duration() as u32 {
                progress_pane.edit(|x| {
                    x.scale = RESULT_PROGRESS_SCALE.value_at(frame as f64);
                    x.alpha = RESULT_PROGRESS_ALPHA.value_at(frame as f64) as u8;
                });
                result_pane.edit(|x| {
                    x.alpha = RESULT_PANE_ALPHA.value_at(frame as f64) as u8;
                });
                power_text.edit(|x| {
                    x.scale = RESULT_POWER_SCALE.value_at(frame as f64);
                });
                state.render_frame(&self.calc_rank_pane, 1)?;
            }

            progress_pane.edit(|x| {
                x.scale = 1.0;
                x.alpha = 0;
            });
            result_pane.edit(|x| {
                x.alpha = 255;
            });
            power_text.edit(|x| {
                x.scale = 1.0;
            });
            state.render_frame(&self.calc_rank_pane, 180)?;
        }

        if let Some(estimated_rank) = estimated_rank {
            state.animate_transition(
                &self.calc_rank_pane,
                &self.calc_rank_pane,
                WINDOW_OUT_SCALE,
                WINDOW_OUT_ALPHA,
                6,
            )?;

            result_pane.edit(|x| x.alpha = 0);
            rank_pane.edit(|x| x.alpha = 255);

            position_text.set_text(get_text(LANG, "position"));
            estimate_text.set_text(get_text(LANG, "estimate"));
            rank_text.set_text(get_text_fmt(
                LANG,
                "rank_value",
                [(FmtKey::Rank, &estimated_rank.to_string())],
            ));

            state.animate_transition(
                &self.calc_rank_pane,
                &self.calc_rank_pane,
                WINDOW_IN_SCALE,
                WINDOW_IN_ALPHA,
                6,
            )?;

            state.animate_transition(
                &self.calc_rank_pane,
                &inner_rank_pane,
                RESULT_RANK_SCALE,
                RESULT_RANK_ALPHA,
                120,
            )?;
        }

        state.animate_transition(
            &self.calc_rank_pane,
            &self.calc_rank_pane,
            WINDOW_OUT_SCALE,
            WINDOW_OUT_ALPHA,
            90,
        )?;
        state.push_frame(0);

        Ok(())
    }

    fn generate_set_played(
        &self,
        matches: [MatchOutcome; 5],
        old_power: f64,
        new_power: f64,
        old_rank: u32,
        new_rank: u32,
    ) -> Result<()> {
        use power_progress_pane::*;

        let mut state = self.state.borrow_mut();
        state.render_frame(&self.change_power_pane, 1)?;

        Ok(())
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
