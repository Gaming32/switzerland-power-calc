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
    pub extra_behavior: ExtraBehavior,
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
        extra_behavior: ExtraBehavior::None,
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
        if let PaneContents::Text(text) = &mut self.contents {
            text.text = new_text.into();
            if let Some(new_color) = new_color.into() {
                text.color = new_color;
            }
        } else {
            panic!("Pane::set_text_and_color called on a non-Text Pane!");
        }
    }

    fn render(
        &self,
        canvas: &mut SurfaceCanvas,
        parent: Option<&Pane>,
        parent_rect: Rect,
        inherited_x_scale: f64,
        inherited_y_scale: f64,
    ) -> Result<()> {
        let local_rect = match self.extra_behavior {
            ExtraBehavior::AdjustToContentBounds {
                sibling,
                min_width,
                margin,
            } => {
                if let Some(parent) = parent
                    && let Some(sibling) = parent.children.iter().find(|x| x.name() == sibling)
                {
                    let (parent_width, parent_height) = parent.rect.size();
                    let sibling = sibling.pane();
                    let mut rect =
                        sibling
                            .contents
                            .bounds_in_parent(sibling.compute_local_draw_rect(
                                Rect::new(0, 0, parent_width, parent_height),
                                sibling.rect,
                                1.0,
                                1.0,
                            ))?;
                    rect.set_x(
                        rect.x()
                            + self.parent_anchor.horizontal.align(parent_width as i32)
                            + self.parent_anchor.horizontal.align(rect.width() as i32),
                    );
                    rect.set_y(
                        -rect.y() + self.parent_anchor.vertical.align(parent_height as i32)
                            - self.parent_anchor.vertical.align(rect.height() as i32),
                    );
                    rect.set_width(rect.width().max(min_width) + margin);
                    rect
                } else {
                    self.rect
                }
            }
            _ => self.rect,
        };
        let draw_rect = self.compute_local_draw_rect(
            parent_rect,
            local_rect,
            inherited_x_scale,
            inherited_y_scale,
        );
        let accumulated_x_scale = inherited_x_scale * self.scale.0;
        let accumulated_y_scale = inherited_y_scale * self.scale.1;

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

    fn compute_local_draw_rect(
        &self,
        parent_rect: Rect,
        local_rect: Rect,
        inherited_x_scale: f64,
        inherited_y_scale: f64,
    ) -> Rect {
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

        let accumulated_x_scale = inherited_x_scale * self.scale.0;
        let accumulated_y_scale = inherited_y_scale * self.scale.1;
        let width = (local_rect.width() as f64 * accumulated_x_scale) as u32;
        let height = (local_rect.height() as f64 * accumulated_y_scale) as u32;
        Rect::new(
            origin_x + (local_rect.x as f64 * inherited_x_scale) as i32
                - self.anchor.horizontal.align(width as i32),
            origin_y
                - (local_rect.y as f64 * inherited_y_scale) as i32
                - self.anchor.vertical.align(height as i32),
            width,
            height,
        )
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
            child.pane().render(
                canvas,
                Some(self),
                draw_rect,
                accumulated_x_scale,
                accumulated_y_scale,
            )?;
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
    Text(TextPaneContents),
    Custom(fn(&mut SurfaceCanvas, Rect) -> Result<()>),
}

impl PaneContents {
    pub fn image_png(bytes: &[u8]) -> Result<Self> {
        Ok(Self::Image(Rc::new(RWops::from_bytes(bytes)?.load_png()?)))
    }

    fn bounds_in_parent(&self, viewport: Rect) -> Result<Rect> {
        Ok(match self {
            PaneContents::Null => Rect::new(
                viewport.x() + (viewport.width() / 2) as i32,
                viewport.y() + (viewport.height() / 2) as i32,
                1,
                1,
            ),
            PaneContents::Image(_) => viewport,
            PaneContents::Text(contents) => {
                let (text_width, text_height) = contents.font.size_of(&contents.text)?;
                contents.compute_rect(text_width, text_height, viewport, 1.0, 1.0)
            }
            PaneContents::Custom(_) => viewport,
        })
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
            PaneContents::Text(contents) => {
                let text_surface = contents.font.render(contents.color, &contents.text)?;
                let rect = contents.compute_rect(
                    text_surface.width(),
                    text_surface.height(),
                    viewport,
                    x_scale,
                    y_scale,
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
pub struct TextPaneContents {
    pub text: Cow<'static, str>,
    pub font: Rc<FontSet<'static>>,
    pub color: Color,
    pub scale: (f64, f64),
    pub secondary_scale: f64,
    pub alignment: Alignment,
}

impl TextPaneContents {
    pub fn new(text: impl Into<Cow<'static, str>>, font: &Rc<FontSet<'static>>) -> Self {
        Self {
            text: text.into(),
            font: font.clone(),
            color: Color::WHITE,
            scale: (1.0, 1.0),
            secondary_scale: 1.0,
            alignment: Alignment::CENTER,
        }
    }

    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }

    pub fn scale(mut self, width: f64, height: f64) -> Self {
        self.scale = (width, height);
        self
    }

    pub fn secondary_scale(mut self, scale: f64) -> Self {
        self.secondary_scale = scale;
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    fn compute_rect(
        &self,
        text_width: u32,
        text_height: u32,
        viewport: Rect,
        parent_x_scale: f64,
        parent_y_scale: f64,
    ) -> Rect {
        let x_scale = self.scale.0 / self.secondary_scale * parent_x_scale;
        let y_scale = self.scale.1 / self.secondary_scale * parent_y_scale;

        let text_rel_x = self.alignment.horizontal.align(viewport.width() as i32)
            - self
                .alignment
                .horizontal
                .align((text_width as f64 * x_scale) as i32);
        let text_rel_y = self.alignment.vertical.align(viewport.height() as i32)
            - self
                .alignment
                .vertical
                .align((text_height as f64 * y_scale) as i32);

        Rect::new(
            viewport.x() + text_rel_x,
            viewport.y() + text_rel_y,
            (text_width as f64 * x_scale) as u32,
            (text_height as f64 * y_scale) as u32,
        )
    }
}

#[derive(Clone)]
pub enum ExtraBehavior {
    None,
    AdjustToContentBounds {
        sibling: &'static str,
        min_width: u32,
        margin: u32,
    },
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

    pub fn set_alpha(&self, alpha: u8) {
        self.edit(|x| x.alpha = alpha);
    }

    pub fn render(&self, canvas: &mut SurfaceCanvas) -> Result<()> {
        let rect = canvas.surface().rect();
        let pane = self.pane();
        pane.render(canvas, None, rect, pane.scale.0, pane.scale.1)
    }

    pub fn immediate_child(&self, name: &str) -> Option<BuiltPane> {
        self.pane()
            .children
            .iter()
            .find(|x| x.name() == name)
            .cloned()
    }

    pub fn child(&self, path: &[&str]) -> Option<BuiltPane> {
        match *path {
            [] => Some(self.clone()),
            [name] => self.immediate_child(name),
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
