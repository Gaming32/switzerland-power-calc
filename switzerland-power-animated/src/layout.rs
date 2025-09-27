use crate::Result;
use crate::alignment::Alignment;
use crate::font::FontSet;
use crate::generator::PIXEL_FORMAT;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::SurfaceCanvas;
use sdl2::surface::Surface;
use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

pub struct Pane {
    pub name: &'static str,
    pub rect: Rect,
    pub scale: f64,
    pub alpha: u8,
    pub anchor: Alignment,
    pub contents: PaneContents,
    pub children: Vec<Rc<RefCell<Pane>>>,
}

impl Default for Pane {
    fn default() -> Self {
        const DEFAULT_RECT: sdl2::sys::SDL_Rect = sdl2::sys::SDL_Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };
        Self {
            name: "",
            rect: DEFAULT_RECT.into(),
            scale: 1.0,
            alpha: 255,
            anchor: Alignment::CENTER,
            contents: PaneContents::Null,
            children: vec![],
        }
    }
}

impl From<Pane> for Rc<RefCell<Pane>> {
    fn from(val: Pane) -> Self {
        Rc::new(RefCell::new(val))
    }
}

impl Pane {
    pub fn build(self) -> BuiltPane {
        BuiltPane(self.into())
    }

    fn render(
        &self,
        canvas: &mut SurfaceCanvas,
        origin_x: i32,
        origin_y: i32,
        parent_scale: f64,
    ) -> Result<()> {
        let accumulated_scale = parent_scale * self.scale;
        let width = (self.rect.width() as f64 * accumulated_scale) as u32;
        let height = (self.rect.height() as f64 * accumulated_scale) as u32;
        let adjusted_rect = Rect::new(
            origin_x + (self.rect.x as f64 * parent_scale) as i32
                - self.anchor.horizontal.align(width as i32),
            origin_y
                - (self.rect.y as f64 * parent_scale) as i32
                - self.anchor.vertical.align(height as i32),
            width,
            height,
        );

        match self.alpha {
            0 => {}
            255 => self.render_internal(canvas, adjusted_rect, accumulated_scale)?,
            alpha => {
                let mut sub_canvas =
                    Surface::new(adjusted_rect.width(), adjusted_rect.height(), PIXEL_FORMAT)?
                        .into_canvas()?;
                let draw_rect = sub_canvas.surface().rect();
                self.render_internal(&mut sub_canvas, draw_rect, accumulated_scale)?;

                sub_canvas.surface_mut().set_alpha_mod(alpha);
                sub_canvas
                    .surface()
                    .blit(None, canvas.surface_mut(), adjusted_rect)?;
            }
        }
        Ok(())
    }

    fn render_internal(
        &self,
        canvas: &mut SurfaceCanvas,
        draw_rect: Rect,
        accumulated_scale: f64,
    ) -> Result<()> {
        let old_scale = canvas.scale();
        canvas.set_scale(accumulated_scale as f32, accumulated_scale as f32)?;

        self.contents.render(canvas, draw_rect, accumulated_scale)?;

        let center = draw_rect.center();
        for child in &self.children {
            let child = child.borrow();
            child.render(canvas, center.x, center.y, accumulated_scale)?;
        }

        canvas.set_scale(old_scale.0, old_scale.1)?;
        Ok(())
    }
}

#[derive(Default)]
pub enum PaneContents {
    #[default]
    Null,
    Image(Surface<'static>),
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
    pub fn set_text(&mut self, new_text: impl Into<Cow<'static, str>>) {
        if let PaneContents::Text { text, .. } = self {
            *text = new_text.into();
        } else {
            panic!("PaneContents::set_text called on a non-Text Pane!");
        }
    }

    fn render(&self, canvas: &mut SurfaceCanvas, viewport: Rect, scale: f64) -> Result<()> {
        match self {
            PaneContents::Null => {}
            PaneContents::Image(image) => {
                image.blit_scaled(None, canvas.surface_mut(), viewport)?;
            }
            PaneContents::Text {
                scale: (x_scale, y_scale),
                text_alignment,
                font,
                color,
                text,
            } => {
                let text_surface = font.render(*color, text)?;

                let x_scale = x_scale * scale;
                let y_scale = y_scale * scale;

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
                        (viewport.x() as f64 / scale) as i32,
                        (viewport.y() as f64 / scale) as i32,
                        (viewport.width() as f64 / scale) as u32,
                        (viewport.height() as f64 / scale) as u32,
                    ),
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct BuiltPane(Rc<RefCell<Pane>>);

impl BuiltPane {
    pub fn pane(&self) -> Ref<'_, Pane> {
        self.0.borrow()
    }

    pub fn edit<R>(&self, editor: impl FnOnce(&mut Pane) -> R) -> R {
        editor(&mut self.0.borrow_mut())
    }

    pub fn render(&self, canvas: &mut SurfaceCanvas) -> Result<()> {
        let center = canvas.surface().rect().center();
        let pane = self.pane();
        pane.render(canvas, center.x, center.y, pane.scale)
    }

    pub fn child(&self, path: &[&str]) -> Option<BuiltPane> {
        if path.is_empty() {
            return Some(self.clone());
        }
        let search_name = *path.first().unwrap();
        self.pane()
            .children
            .iter()
            .cloned()
            .map(BuiltPane)
            .filter_map(|x| {
                (x.pane().name == search_name)
                    .then(|| x.child(&path[1..]))
                    .flatten()
            })
            .next()
    }
}
