use crate::Result;
use crate::alignment::Alignment;
use crate::font::FontSet;
use crate::generator::PIXEL_FORMAT;
use sdl2::image::ImageRWops;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::SurfaceCanvas;
use sdl2::rwops::RWops;
use sdl2::surface::Surface;
use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::mem;
use std::rc::Rc;

#[derive(Clone)]
pub struct Pane {
    pub name: &'static str,
    pub rect: Rect,
    pub scale: (f64, f64),
    pub alpha: u8,
    pub anchor: Alignment,
    pub parent_anchor: Alignment,
    pub contents: PaneContents,
    pub children: Vec<BuiltPane>,
}

impl Default for Pane {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Pane {
    pub const EMPTY: Self = Self {
        name: "",
        rect: unsafe {
            mem::transmute::<sdl2::sys::SDL_Rect, Rect>(sdl2::sys::SDL_Rect {
                x: 0,
                y: 0,
                w: 30,
                h: 40,
            })
        },
        scale: (1.0, 1.0),
        alpha: 255,
        anchor: Alignment::CENTER,
        parent_anchor: Alignment::CENTER,
        contents: PaneContents::Null,
        children: vec![],
    };

    pub fn build(self) -> BuiltPane {
        BuiltPane(self.name, Rc::new(RefCell::new(self)))
    }

    pub fn set_scale(&mut self, new_scale: f64) {
        self.scale = (new_scale, new_scale);
    }

    pub fn set_text(&mut self, new_text: impl Into<Cow<'static, str>>) {
        self.set_text_and_color(new_text, None);
    }

    pub fn set_text_and_color(
        &mut self,
        new_text: impl Into<Cow<'static, str>>,
        new_color: impl Into<Option<Color>>,
    ) {
        if let PaneContents::Text { text, color, .. } = &mut self.contents {
            *text = new_text.into();
            if let Some(new_color) = new_color.into() {
                *color = new_color;
            }
        } else {
            panic!("PaneContents::set_text called on a non-Text Pane!");
        }
    }

    fn render(
        &self,
        canvas: &mut SurfaceCanvas,
        parent_rect: Rect,
        parent_x_scale: f64,
        parent_y_scale: f64,
    ) -> Result<()> {
        let origin_x = parent_rect.x()
            + self
                .parent_anchor
                .horizontal
                .align(parent_rect.width() as i32);
        let origin_y = parent_rect.y()
            + self
                .parent_anchor
                .vertical
                .align(parent_rect.height() as i32);

        let accumulated_x_scale = parent_x_scale * self.scale.0;
        let accumulated_y_scale = parent_y_scale * self.scale.1;
        let width = (self.rect.width() as f64 * accumulated_x_scale) as u32;
        let height = (self.rect.height() as f64 * accumulated_y_scale) as u32;
        let draw_rect = Rect::new(
            origin_x + (self.rect.x as f64 * parent_x_scale) as i32
                - self.anchor.horizontal.align(width as i32),
            origin_y
                - (self.rect.y as f64 * parent_y_scale) as i32
                - self.anchor.vertical.align(height as i32),
            width,
            height,
        );

        match self.alpha {
            0 => {}
            255 => {
                self.render_internal(canvas, draw_rect, accumulated_x_scale, accumulated_y_scale)?
            }
            alpha => {
                let mut sub_canvas = Surface::new(
                    canvas.surface().width(),
                    canvas.surface().height(),
                    PIXEL_FORMAT,
                )?
                .into_canvas()?;
                self.render_internal(
                    &mut sub_canvas,
                    draw_rect,
                    accumulated_x_scale,
                    accumulated_y_scale,
                )?;
                sub_canvas.surface_mut().set_alpha_mod(alpha);
                sub_canvas
                    .surface()
                    .blit(None, canvas.surface_mut(), None)?;
            }
        }
        Ok(())
    }

    fn render_internal(
        &self,
        canvas: &mut SurfaceCanvas,
        draw_rect: Rect,
        accumulated_x_scale: f64,
        accumulated_y_scale: f64,
    ) -> Result<()> {
        let old_scale = canvas.scale();
        canvas.set_scale(accumulated_x_scale as f32, accumulated_y_scale as f32)?;

        self.contents
            .render(canvas, draw_rect, accumulated_x_scale, accumulated_y_scale)?;

        for child in &self.children {
            child
                .pane()
                .render(canvas, draw_rect, accumulated_x_scale, accumulated_y_scale)?;
        }

        canvas.set_scale(old_scale.0, old_scale.1)?;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub enum PaneContents {
    #[default]
    Null,
    Image(Rc<Surface<'static>>),
    Text {
        text: Cow<'static, str>,
        font: Rc<FontSet<'static>>,
        color: Color,
        scale: (f64, f64),
        text_alignment: Alignment,
    },
    Custom(&'static dyn Fn(&mut SurfaceCanvas, Rect) -> Result<()>),
}

impl PaneContents {
    pub fn image_png(bytes: &[u8]) -> Result<Self> {
        Ok(Self::Image(Rc::new(RWops::from_bytes(bytes)?.load_png()?)))
    }

    fn render(
        &self,
        canvas: &mut SurfaceCanvas,
        viewport: Rect,
        x_scale: f64,
        y_scale: f64,
    ) -> Result<()> {
        match self {
            PaneContents::Null => {}
            PaneContents::Image(image) => {
                image.blit_scaled(None, canvas.surface_mut(), viewport)?;
            }
            PaneContents::Text {
                scale: (text_x_scale, text_y_scale),
                text_alignment,
                font,
                color,
                text,
            } => {
                let text_surface = font.render(*color, text)?;

                let x_scale = text_x_scale * x_scale;
                let y_scale = text_y_scale * y_scale;

                let text_rel_x = text_alignment.horizontal.align(viewport.width() as i32)
                    - text_alignment
                        .horizontal
                        .align((text_surface.width() as f64 * x_scale) as i32);
                let text_rel_y = text_alignment.vertical.align(viewport.height() as i32)
                    - text_alignment
                        .vertical
                        .align((text_surface.height() as f64 * y_scale) as i32);

                let rect = Rect::new(
                    viewport.x() + text_rel_x,
                    viewport.y() + text_rel_y,
                    (text_surface.width() as f64 * x_scale) as u32,
                    (text_surface.height() as f64 * y_scale) as u32,
                );
                text_surface.blit_scaled(None, canvas.surface_mut(), rect)?;
            }
            PaneContents::Custom(renderer) => {
                renderer(
                    canvas,
                    Rect::new(
                        (viewport.x() as f64 / x_scale) as i32,
                        (viewport.y() as f64 / y_scale) as i32,
                        (viewport.width() as f64 / x_scale) as u32,
                        (viewport.height() as f64 / y_scale) as u32,
                    ),
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct BuiltPane(&'static str, Rc<RefCell<Pane>>);

impl BuiltPane {
    pub fn name(&self) -> &'static str {
        self.0
    }

    pub fn pane(&self) -> Ref<'_, Pane> {
        self.1.borrow()
    }

    pub fn edit<R>(&self, editor: impl FnOnce(&mut Pane) -> R) -> R {
        editor(&mut self.1.borrow_mut())
    }

    pub fn set_text(&self, new_text: impl Into<Cow<'static, str>>) {
        self.edit(|x| x.set_text(new_text))
    }

    pub fn set_text_and_color(
        &self,
        new_text: impl Into<Cow<'static, str>>,
        new_color: impl Into<Option<Color>>,
    ) {
        self.edit(|x| x.set_text_and_color(new_text, new_color))
    }

    pub fn render(&self, canvas: &mut SurfaceCanvas) -> Result<()> {
        let rect = canvas.surface().rect();
        let pane = self.pane();
        pane.render(canvas, rect, pane.scale.0, pane.scale.1)
    }

    pub fn child(&self, path: &[&str]) -> Option<BuiltPane> {
        match *path {
            [] => Some(self.clone()),
            [name] => self
                .pane()
                .children
                .iter()
                .find(|x| x.name() == name)
                .cloned(),
            [name, ..] => self
                .pane()
                .children
                .iter()
                .filter(|x| x.name() == name)
                .filter_map(|x| x.child(&path[1..]))
                .next(),
        }
    }

    pub fn children(&self, path: &[&str]) -> Vec<BuiltPane> {
        match *path {
            [] => vec![self.clone()],
            [name] => self
                .pane()
                .children
                .iter()
                .filter(|x| x.name() == name)
                .cloned()
                .collect(),
            [name, ..] => self
                .pane()
                .children
                .iter()
                .filter(|x| x.name() == name)
                .flat_map(|x| x.children(&path[1..]))
                .collect(),
        }
    }
}
