use crate::Result;
use crate::surface::ScratchSurface;
use itertools::Itertools;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::rwops::RWops;
use sdl2::surface::Surface;
use sdl2::ttf::{Font, Sdl2TtfContext};
use smallvec::SmallVec;

pub struct FontSet<'ttf> {
    pub fonts: SmallVec<[Font<'ttf, 'static>; 2]>,
}

impl<'ttf> FontSet<'ttf> {
    pub fn load(
        context: &'ttf Sdl2TtfContext,
        point_size: u16,
        fonts: &[&'static [u8]],
    ) -> Result<Self> {
        if fonts.is_empty() {
            panic!("At least one font is required for FontSet");
        }
        Ok(Self {
            fonts: fonts
                .iter()
                .map(|font| Ok(context.load_font_from_rwops(RWops::from_bytes(font)?, point_size)?))
                .collect::<Result<_>>()?,
        })
    }

    pub fn size_of(&self, text: &str) -> Result<(u32, u32)> {
        Ok(match self.split_text(text)? {
            SplitTextResult::EmptyText => (0, 0),
            SplitTextResult::SingleFont(font) => font.size_of(text)?,
            SplitTextResult::MultipleFonts(_, size) => size,
        })
    }

    /// Renders the specified text into the specified [Surface] at 0,0 with no blending and returns
    /// the blitted-to [Rect], if any.
    pub fn render(
        &self,
        dest: &mut Surface,
        x_scale: f64,
        y_scale: f64,
        color: impl Into<Color>,
        text: &str,
    ) -> Result<Option<Rect>> {
        Ok(match self.split_text(text)? {
            SplitTextResult::EmptyText => None,
            SplitTextResult::SingleFont(font) => {
                let mut result = font.render(text).blended(color)?;
                let scaled_width = (result.width() as f64 * x_scale) as u32;
                let scaled_height = (result.height() as f64 * y_scale) as u32;
                Some(result.blit_smooth(dest, Rect::new(0, 0, scaled_width, scaled_height))?)
            }
            SplitTextResult::MultipleFonts(segments, (width, height)) => {
                let scaled_width = (width as f64 * x_scale) as u32;
                let scaled_height = (height as f64 * y_scale) as u32;
                let dest_rect = Rect::new(0, 0, scaled_width, scaled_height);

                let color = color.into();
                let max_ascent = segments
                    .iter()
                    .map(|(font, _)| font.ascent())
                    .max()
                    .unwrap();

                let mut cur_x = 0;
                for (font, segment) in segments {
                    let mut segment = font.render(segment).blended(color)?;
                    let scaled_width = (segment.width() as f64 * x_scale) as u32;
                    let scaled_height = (segment.height() as f64 * y_scale) as u32;
                    segment.blit_smooth(
                        dest,
                        Rect::new(
                            cur_x,
                            ((max_ascent - font.ascent()) as f64 * y_scale) as i32,
                            scaled_width,
                            scaled_height,
                        ),
                    )?;
                    cur_x += scaled_width as i32;
                }

                Some(dest_rect)
            }
        })
    }

    fn split_text<'font, 'str>(
        &'font self,
        text: &'str str,
    ) -> Result<SplitTextResult<'font, 'str, 'ttf>> {
        if text.is_empty() {
            return Ok(SplitTextResult::EmptyText);
        }

        Ok(
            match text
                .char_indices()
                .chunk_by(|(_, ch)| {
                    self.fonts
                        .iter()
                        .position(|x| x.find_glyph(*ch).is_some())
                        .unwrap_or_default()
                })
                .into_iter()
                .map(|(font_index, mut indices)| {
                    let (start_index, start_ch) = indices.next().unwrap();
                    let end_index = indices.last().map_or_else(
                        || start_index + start_ch.len_utf8(),
                        |(x, ch)| x + ch.len_utf8(),
                    );
                    (&self.fonts[font_index], &text[start_index..end_index])
                })
                .exactly_one()
            {
                Ok((font, _)) => SplitTextResult::SingleFont(font),
                Err(segments) => {
                    let segments = segments.collect::<SmallVec<[_; 4]>>();
                    let size = segments
                        .iter()
                        .map(|(font, segment)| font.size_of(segment))
                        .fold_ok((0, 0), |(old_w, old_h), (new_w, new_h)| {
                            (old_w + new_w, old_h.max(new_h))
                        })?;
                    SplitTextResult::MultipleFonts(segments, size)
                }
            },
        )
    }
}

enum SplitTextResult<'font, 'str, 'ttf> {
    EmptyText,
    SingleFont(&'font Font<'ttf, 'static>),
    MultipleFonts(
        SmallVec<[(&'font Font<'ttf, 'static>, &'str str); 4]>,
        (u32, u32),
    ),
}
