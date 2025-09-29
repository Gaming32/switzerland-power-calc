use crate::Result;
use crate::alignment::Alignment;
use crate::animation::AnimationSet;
use crate::font::FontSet;
use crate::layout::{BuiltPane, PaneContents};
use crate::panes::{calc_rank_pane, power_progress_pane};
use crate::status::SetScore;
use crate::texts::{format_power, format_rank, get_text};
use crate::{MatchOutcome, PowerStatus};
use itertools::izip;
use sdl2::Sdl;
use sdl2::image::{InitFlag, Sdl2ImageContext};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{BlendMode, SurfaceCanvas};
use sdl2::surface::Surface;
use sdl2::ttf::Sdl2TtfContext;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::mem;
use std::rc::Rc;
use webp::{AnimEncoder, AnimFrame, WebPConfig, WebPMemory};

pub const WIDTH: u32 = 1250;
pub const HEIGHT: u32 = 776;
pub const SWITZERLAND_COLOR: Color = Color::RGB(218, 41, 28);

const FPS: u32 = 60;
const LANG: &str = "JPja"; // TODO: Make configurable

pub(crate) const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::ABGR8888;

pub struct AnimationGenerator {
    state: RefCell<GeneratorState>,

    calc_rank_pane: BuiltPane,
    power_progress_pane: BuiltPane,

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
        let bold_font_small = Rc::new(load_font!(36, "BlitzBold.otf", "FOT-RowdyStd-EB.otf"));

        let swiss_flag = PaneContents::image_png(include_bytes!("panes/images/swiss-flag.png"))?;

        Ok(Self {
            state: RefCell::new(GeneratorState {
                canvas: Surface::new(WIDTH, HEIGHT, PIXEL_FORMAT)?.into_canvas()?,
                frames: vec![],
            }),

            calc_rank_pane: calc_rank_pane::calc_rank_pane(bold_font.clone(), swiss_flag.clone())?,
            power_progress_pane: power_progress_pane::power_progress_pane(
                bold_font.clone(),
                bold_font_small.clone(),
                swiss_flag.clone(),
            )?,

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
            PowerStatus::SetPlayed {
                matches,
                old_power,
                new_power,
                old_rank,
                new_rank,
            } => {
                self.generate_set_played(matches, old_power, new_power, old_rank, new_rank)?;
            }
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

        let progress_pane = self
            .calc_rank_pane
            .immediate_child("progress_pane")
            .unwrap();
        let progress_text = progress_pane.immediate_child("progress_text").unwrap();

        let result_pane = self.calc_rank_pane.immediate_child("result_pane").unwrap();
        let power_value_text = result_pane.immediate_child("power_value_text").unwrap();

        let rank_pane = self.calc_rank_pane.immediate_child("rank_pane").unwrap();
        let inner_rank_pane = rank_pane.immediate_child("inner_rank_pane").unwrap();

        self.calc_rank_pane.edit(|x| {
            x.set_scale(1.0);
            x.alpha = 255;
        });

        progress_pane
            .immediate_child("calculating_text")
            .unwrap()
            .set_text(get_text(LANG, "calculating"));
        progress_pane.edit(|x| {
            x.set_scale(1.0);
            x.alpha = 255;
        });
        progress_text.edit(|x| {
            x.set_scale(1.0);
            x.alpha = 255;
            x.set_text((progress - 1).to_string());
        });
        progress_pane
            .immediate_child("total_text")
            .unwrap()
            .set_text(format!("/{total}"));

        result_pane.set_alpha(0);
        power_value_text.edit(|x| {
            x.set_scale(1.0);
        });

        rank_pane.set_alpha(0);
        inner_rank_pane.edit(|x| {
            x.set_scale(1.0);
            x.alpha = 0;
        });

        state.animate(&self.calc_rank_pane, WINDOW_IN, 60)?;

        progress_text.set_alpha(0);
        state.render_frame(&self.calc_rank_pane, 1)?;

        progress_text.set_text(progress.to_string());
        state.animate(&self.calc_rank_pane, PROGRESS_IN, 120)?;

        if let Some(calculated_power) = calculated_power {
            result_pane
                .immediate_child("calculated_text")
                .unwrap()
                .set_text(get_text(LANG, "calculated"));
            power_value_text.set_text(format_power(LANG, "power_value", calculated_power));
            state.animate(&self.calc_rank_pane, RESULT_POWER_IN, 180)?;
        }

        if let Some(estimated_rank) = estimated_rank {
            state.animate(&self.calc_rank_pane, WINDOW_OUT, 6)?;

            result_pane.set_alpha(0);
            rank_pane.set_alpha(255);

            rank_pane
                .immediate_child("position_text")
                .unwrap()
                .set_text(get_text(LANG, "position"));
            rank_pane
                .immediate_child("estimate_text")
                .unwrap()
                .set_text(get_text(LANG, "estimate"));
            inner_rank_pane
                .immediate_child("rank_value_text")
                .unwrap()
                .set_text(format_rank(LANG, estimated_rank));

            state.animate(&self.calc_rank_pane, WINDOW_IN, 6)?;
            state.animate(&self.calc_rank_pane, RESULT_RANK_IN, 120)?;
        }

        state.animate(&self.calc_rank_pane, WINDOW_OUT, 90)?;

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

        let (wins, losses) = matches.set_score();

        let mut state = self.state.borrow_mut();

        let set_outcome_pane = self
            .power_progress_pane
            .immediate_child("set_outcome_pane")
            .unwrap();
        let set_score_text = set_outcome_pane.immediate_child("set_score_text").unwrap();
        let win_lose_panes = set_outcome_pane.children(&["win_lose_pane"]);
        let win_lose_animation_panes =
            set_outcome_pane.children(&["win_lose_pane", "animation_pane"]);
        let win_lose_texts =
            set_outcome_pane.children(&["win_lose_pane", "animation_pane", "text"]);

        let power_pane = self
            .power_progress_pane
            .immediate_child("power_pane")
            .unwrap();
        let power_text = power_pane.immediate_child("power_text").unwrap();
        let power_value_text = power_pane.immediate_child("power_value_text").unwrap();

        power_text.set_text(get_text(LANG, "power"));
        power_value_text.set_text(format_power(LANG, "power_value", old_power));

        state.animate(&self.power_progress_pane, WINDOW_IN, 1)?;

        let win_text = get_text(LANG, "win");
        let lose_text = get_text(LANG, "lose");
        for (outcome, base_pane, animation_pane, text_pane, pos) in izip!(
            matches.into_iter().filter(|x| *x != MatchOutcome::Unplayed),
            win_lose_panes.iter(),
            win_lose_animation_panes.iter(),
            win_lose_texts.iter(),
            WIN_LOSE_POSITIONS[wins + losses - 2]
        ) {
            let (color, text) = match outcome {
                MatchOutcome::Win => (WIN_COLOR, win_text),
                MatchOutcome::Lose => (LOSE_COLOR, lose_text),
                MatchOutcome::Unplayed => unreachable!(),
            };

            base_pane.edit(|x| x.rect.reposition(pos));
            text_pane.set_text_and_color(text, color);

            state.animate_specific_pane(
                &self.power_progress_pane,
                animation_pane,
                WIN_LOSE_IN,
                0,
            )?;
        }
        state.push_frame(60);

        set_score_text.set_text(format!("{wins} - {losses}"));
        state.animate(&self.power_progress_pane, SET_SCORE_IN, 60)?;

        let power_change = format_power(
            LANG,
            if new_power >= old_power {
                "power_up"
            } else {
                "power_down"
            },
            new_power - old_power,
        );

        power_pane
            .child(&["power_diff", "value"])
            .unwrap()
            .set_text(power_change.clone());
        state.animate(&self.power_progress_pane, POWER_DIFF_IN, 2)?;

        power_pane
            .immediate_child("point_diff_anim")
            .unwrap()
            .set_text(power_change);
        state.animate(&self.power_progress_pane, POWER_ADD, 1)?;

        state.animate_value_change(
            &self.power_progress_pane,
            &power_value_text,
            old_power,
            new_power,
            |distance| distance.powf(0.1).max(distance / 180.0),
            |power| format_power(LANG, "power_value", power),
            || Ok(()),
            30,
        )?;

        state.animate(&self.power_progress_pane, WINDOW_OUT, 2)?;

        self.generate_rank_change(&mut state, old_rank, new_rank)?;

        state.push_frame(0);
        Ok(())
    }

    fn generate_rank_change(
        &self,
        state: &mut GeneratorState,
        old_rank: u32,
        new_rank: u32,
    ) -> Result<()> {
        use calc_rank_pane::*;

        let rank_pane = self.calc_rank_pane.immediate_child("rank_pane").unwrap();
        let inner_rank_pane = rank_pane.immediate_child("inner_rank_pane").unwrap();
        let rank_arrow_root = inner_rank_pane.immediate_child("rank_arrow_root").unwrap();
        let rank_arrow_root_inner = rank_arrow_root.child(&["inner", "inner_inner"]).unwrap();

        self.calc_rank_pane
            .immediate_child("progress_pane")
            .unwrap()
            .set_alpha(0);
        self.calc_rank_pane
            .immediate_child("result_pane")
            .unwrap()
            .set_alpha(0);
        rank_pane.set_alpha(255);
        inner_rank_pane.set_alpha(255);

        rank_pane
            .immediate_child("position_text")
            .unwrap()
            .set_text(get_text(LANG, "position"));
        rank_pane
            .immediate_child("estimate_text")
            .unwrap()
            .set_text(get_text(LANG, "estimate"));
        inner_rank_pane
            .immediate_child("rank_value_text")
            .unwrap()
            .set_text(format_rank(LANG, old_rank));

        rank_arrow_root_inner.edit(|x| x.rect.set_x(if new_rank == old_rank { 20 } else { 0 }));
        let rank_arrow_name = match new_rank.cmp(&old_rank) {
            Ordering::Equal => "rank_stay_arrow",
            Ordering::Less => "rank_up_arrow",
            Ordering::Greater => "rank_down_arrow",
        };
        for rank_arrow in &rank_arrow_root_inner.pane().children {
            if rank_arrow.name() == rank_arrow_name {
                rank_arrow.set_alpha(255);
            } else {
                rank_arrow.set_alpha(0);
            }
        }

        state.animate(&self.calc_rank_pane, WINDOW_IN, 0)?;

        state.animate_value_change(
            &self.calc_rank_pane,
            &inner_rank_pane.immediate_child("rank_value_text").unwrap(),
            old_rank as f64,
            new_rank as f64,
            |distance| distance / 120.0,
            |rank| format_rank(LANG, rank.round() as u32),
            || {
                let mut new_rect = inner_rank_pane
                    .immediate_child_content_bounds("rank_value_text", Alignment::CENTER)?
                    .unwrap();
                new_rect.set_width(new_rect.width().max(300) + 80);
                rank_arrow_root.edit(|x| x.rect = new_rect);
                Ok(())
            },
            120,
        )?;

        state.animate(&self.calc_rank_pane, WINDOW_OUT, 90)?;

        Ok(())
    }
}

type FramesVec = Vec<(Vec<u8>, u32)>;

struct GeneratorState {
    canvas: SurfaceCanvas<'static>,
    frames: FramesVec,
}

impl GeneratorState {
    fn animate<const N: usize>(
        &mut self,
        root_pane: &BuiltPane,
        animation: AnimationSet<N>,
        end_delay: u32,
    ) -> Result<()> {
        self.animate_specific_pane(root_pane, root_pane, animation, end_delay)
    }

    fn animate_specific_pane<const N: usize>(
        &mut self,
        render_pane: &BuiltPane,
        origin_pane: &BuiltPane,
        animation: AnimationSet<N>,
        end_delay: u32,
    ) -> Result<()> {
        animation.animate(render_pane, origin_pane, end_delay, |pane, duration| {
            self.render_frame(pane, duration)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn animate_value_change(
        &mut self,
        render_pane: &BuiltPane,
        value_pane: &BuiltPane,
        old_value: f64,
        new_value: f64,
        change_per_frame: impl FnOnce(f64) -> f64,
        formatter: impl Fn(f64) -> String,
        each_frame: impl Fn() -> Result<()>,
        end_delay: u32,
    ) -> Result<()> {
        let distance = (new_value - old_value).abs();
        let change_per_frame = change_per_frame(distance);

        let mut render = |value, duration| {
            value_pane.set_text(formatter(value));
            each_frame()?;
            self.render_frame(render_pane, duration)
        };

        let mut display_value = old_value;
        if new_value >= old_value {
            loop {
                display_value += change_per_frame;
                if display_value >= new_value {
                    break;
                }
                render(display_value, 1)?;
            }
        } else {
            loop {
                display_value -= change_per_frame;
                if display_value <= new_value {
                    break;
                }
                render(display_value, 1)?;
            }
        }
        render(new_value, end_delay)?;

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
