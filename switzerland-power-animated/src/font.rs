use crate::Result;
use sdl2::rwops::RWops;
use sdl2::ttf::{Font, Sdl2TtfContext};

pub(crate) struct FontPair<'ttf> {
    pub main: Font<'ttf, 'static>,
    pub fallback: Font<'ttf, 'static>,
}

impl<'ttf> FontPair<'ttf> {
    pub fn load(
        context: &'ttf Sdl2TtfContext,
        main: &'static [u8],
        fallback: &'static [u8],
        point_size: u16,
    ) -> Result<Self> {
        Ok(Self {
            main: context.load_font_from_rwops(RWops::from_bytes(main)?, point_size)?,
            fallback: context.load_font_from_rwops(RWops::from_bytes(fallback)?, point_size)?,
        })
    }
}
