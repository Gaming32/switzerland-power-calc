use crate::Result;
use crate::alignment::Alignment;
use crate::font::FontSet;
use crate::generator::PIXEL_FORMAT;
use crate::surface::ScratchSurface;
use sdl2::image::ImageRWops;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, SurfaceCanvas};
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
    pub contents_blending: BlendMode,
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
        contents_blending: BlendMode::Blend,
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

    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        canvas: &mut SurfaceCanvas,
        scratch_surface: &mut Surface,
        parent: Option<&Pane>,
        parent_rect: Rect,
        inherited_x_scale: f64,
        inherited_y_scale: f64,
        inherited_alpha: f64,
    ) -> Result<()> {
        let accumulated_alpha = inherited_alpha * (self.alpha as f64 / 255.0);
        let alpha_8 = (accumulated_alpha * 255.0) as u8;
        if alpha_8 == 0 {
            return Ok(());
        }

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

        self.contents.render(
            canvas,
            scratch_surface,
            draw_rect,
            accumulated_x_scale,
            accumulated_y_scale,
            alpha_8,
            self.contents_blending,
        )?;

        for child in &self.children {
            child.pane().render(
                canvas,
                scratch_surface,
                Some(self),
                draw_rect,
                accumulated_x_scale,
                accumulated_y_scale,
                accumulated_alpha,
            )?;
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
}

#[derive(Clone, Default)]
pub enum PaneContents {
    #[default]
    Null,
    Image(Rc<RefCell<Surface<'static>>>),
    Text(TextPaneContents),
    Custom(fn(canvas: &mut SurfaceCanvas, rect: Rect, alpha: u8) -> Result<()>),
}

impl PaneContents {
    pub fn image_png(bytes: &[u8]) -> Result<Self> {
        Ok(Self::Image(Rc::new(RefCell::new(
            RWops::from_bytes(bytes)?
                .load_png()?
                .convert_format(PIXEL_FORMAT)?,
        ))))
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
                contents.compute_rect(text_width, text_height, viewport)
            }
            PaneContents::Custom(_) => viewport,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn render(
        &self,
        canvas: &mut SurfaceCanvas,
        scratch_surface: &mut Surface,
        viewport: Rect,
        x_scale: f64,
        y_scale: f64,
        alpha: u8,
        blend_mode: BlendMode,
    ) -> Result<()> {
        match self {
            PaneContents::Null => {}
            PaneContents::Image(image) => {
                let mut image = image.borrow_mut();
                if alpha == 255 && blend_mode == BlendMode::None {
                    image.set_alpha_mod(255);
                    image.blit_smooth(canvas.surface_mut(), viewport)?;
                } else {
                    image.blit_smooth(scratch_surface, viewport)?;
                    scratch_surface.set_alpha_mod(alpha);
                    scratch_surface.set_blend_mode(blend_mode)?;
                    scratch_surface.blit(viewport, canvas.surface_mut(), viewport)?;
                }
            }
            PaneContents::Text(contents) => {
                let x_scale = contents.scale.0 / contents.secondary_scale * x_scale;
                let y_scale = contents.scale.1 / contents.secondary_scale * y_scale;
                if let Some(from_rect) = contents.font.render(
                    scratch_surface,
                    x_scale,
                    y_scale,
                    contents.color,
                    &contents.text,
                )? {
                    scratch_surface.set_alpha_mod(alpha);
                    scratch_surface.set_blend_mode(blend_mode)?;
                    scratch_surface.blit(
                        from_rect,
                        canvas.surface_mut(),
                        contents.compute_rect(from_rect.width(), from_rect.height(), viewport),
                    )?;
                }
            }
            PaneContents::Custom(renderer) => {
                let old_scale = canvas.scale();
                canvas.set_scale(x_scale as f32, y_scale as f32)?;
                canvas.set_blend_mode(blend_mode);
                renderer(
                    canvas,
                    Rect::new(
                        (viewport.x() as f64 / x_scale) as i32,
                        (viewport.y() as f64 / y_scale) as i32,
                        (viewport.width() as f64 / x_scale) as u32,
                        (viewport.height() as f64 / y_scale) as u32,
                    ),
                    alpha,
                )?;
                canvas.set_scale(old_scale.0, old_scale.1)?;
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

    fn compute_rect(&self, text_width: u32, text_height: u32, viewport: Rect) -> Rect {
        let text_rel_x = self.alignment.horizontal.align(viewport.width() as i32)
            - self.alignment.horizontal.align(text_width as i32);
        let text_rel_y = self.alignment.vertical.align(viewport.height() as i32)
            - self.alignment.vertical.align(text_height as i32);

        Rect::new(
            viewport.x() + text_rel_x,
            viewport.y() + text_rel_y,
            text_width,
            text_height,
        )
    }
}

#[derive(Copy, Clone)]
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

    pub fn render(&self, canvas: &mut SurfaceCanvas, scratch_surface: &mut Surface) -> Result<()> {
        let rect = canvas.surface().rect();
        let pane = self.pane();
        pane.render(
            canvas,
            scratch_surface,
            None,
            rect,
            pane.scale.0,
            pane.scale.1,
            1.0,
        )
    }

    pub fn deep_clone(&self) -> BuiltPane {
        let pane = self.pane();
        BuiltPane(
            self.0,
            Rc::new(RefCell::new(Pane {
                contents: pane.contents.clone(),
                children: pane.children.iter().map(BuiltPane::deep_clone).collect(),
                ..*pane
            })),
        )
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
