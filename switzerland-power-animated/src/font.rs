use crate::Result;
use itertools::Itertools;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::rwops::RWops;
use sdl2::surface::Surface;
use sdl2::ttf::{Font, Sdl2TtfContext};
use smallvec::SmallVec;

pub(crate) struct FontSet<'ttf> {
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

    pub fn render(&self, color: impl Into<Color>, text: &str) -> Result<Surface<'_>> {
        if text.is_empty() {
            return Ok(self.fonts.first().unwrap().render(text).blended(color)?);
        }

        if let Ok(ch) = text.chars().exactly_one() {
            let font = self
                .fonts
                .iter()
                .find_or_first(|x| x.find_glyph(ch).is_some())
                .unwrap();
            return Ok(font.render_char(ch).blended(color)?);
        }

        let result = match text
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
            Ok((font, _)) => font.render(text).blended(color)?,
            Err(segments) => {
                let color = color.into();
                let segments = segments.collect::<SmallVec<[_; 4]>>();
                let (width, height) = segments
                    .iter()
                    .map(|(font, segment)| font.size_of(segment))
                    .fold_ok((0, 0), |(old_w, old_h), (new_w, new_h)| {
                        (old_w + new_w, old_h.max(new_h))
                    })?;
                let max_ascent = segments
                    .iter()
                    .map(|(font, _)| font.ascent())
                    .max()
                    .unwrap();

                let mut result = Surface::new(width, height, PixelFormatEnum::RGBA8888)?;
                let mut cur_x = 0;
                for (font, segment) in segments {
                    let rendered_segment = font.render(segment).blended(color)?;
                    rendered_segment.blit(
                        None,
                        &mut result,
                        Rect::new(
                            cur_x,
                            max_ascent - font.ascent(),
                            rendered_segment.width(),
                            rendered_segment.height(),
                        ),
                    )?;
                    cur_x += rendered_segment.width() as i32;
                }
                result
            }
        };
        Ok(result)
    }
}
