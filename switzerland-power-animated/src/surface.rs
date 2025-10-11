use sdl2::rect::Rect;
use sdl2::render::BlendMode;
use sdl2::surface::SurfaceRef;

pub trait ScratchSurface {
    fn blit_smooth(
        &mut self,
        from_surface: &mut SurfaceRef,
        dest_rect: Rect,
    ) -> Result<Rect, String>;
}

impl ScratchSurface for SurfaceRef {
    /// Blits to the other surface, without blending. This [Surface]'s blend mode *may* be changed
    /// to [BlendMode::None] in the process.
    fn blit_smooth(&mut self, dst: &mut SurfaceRef, dst_rect: Rect) -> Result<Rect, String> {
        if self.width() != dst_rect.width() || self.height() != dst_rect.height() {
            let use_surface = if get_format(self) != get_format(dst) {
                Some(self.convert(&dst.pixel_format())?)
            } else {
                None
            };
            let use_surface = use_surface.as_deref().unwrap_or(self);
            if let Some((src_rect, dst_rect)) = blit_clip(
                use_surface.rect(),
                dst_rect,
                self.rect(),
                dst.rect(),
                dst.clip_rect().unwrap(),
            ) {
                unsafe {
                    use_surface.soft_stretch_linear(src_rect, dst, dst_rect)?;
                }
            }
        } else {
            self.set_blend_mode(BlendMode::None)?;
            self.blit(None, dst, dst_rect)?;
        }
        Ok(dst_rect)
    }
}

#[inline(always)]
fn get_format(surface: &SurfaceRef) -> u32 {
    unsafe { (*surface.pixel_format().raw()).format }
}

// From SDL_PrivateUpperBlitScaled, which is private and hiding useful functionality
fn blit_clip(
    src_rect: Rect,
    dst_rect: Rect,
    src: Rect,
    dst: Rect,
    dst_clip_rect: Rect,
) -> Option<(Rect, Rect)> {
    let scaling_w = dst_rect.w as f64 / src_rect.w as f64;
    let scaling_h = dst_rect.h as f64 / src_rect.h as f64;

    let mut dst_x0 = dst_rect.x as f64;
    let mut dst_y0 = dst_rect.y as f64;
    let mut dst_x1 = dst_x0 + dst_rect.w as f64;
    let mut dst_y1 = dst_y0 + dst_rect.h as f64;

    let mut src_x0 = src_rect.x as f64;
    let mut src_y0 = src_rect.y as f64;
    let mut src_x1 = src_x0 + src_rect.w as f64;
    let mut src_y1 = src_y0 + src_rect.h as f64;

    if src_x0 < 0.0 {
        dst_x0 -= src_x0 * scaling_w;
        src_x0 = 0.0;
    }

    if src_x1 > src.w as f64 {
        dst_x1 -= (src_x1 - src.w as f64) * scaling_w;
        src_x1 = src.w as f64;
    }

    if src_y0 < 0.0 {
        dst_y0 -= src_y0 * scaling_h;
        src_y0 = 0.0;
    }

    if src_y1 > src.h as f64 {
        dst_y1 -= (src_y1 - src.h as f64) * scaling_h;
        src_y1 = src.h as f64;
    }

    dst_x0 -= dst_clip_rect.x as f64;
    dst_x1 -= dst_clip_rect.x as f64;
    dst_y0 -= dst_clip_rect.y as f64;
    dst_y1 -= dst_clip_rect.y as f64;

    if dst_x0 < 0.0 {
        src_x0 -= dst_x0 / scaling_w;
        dst_x0 = 0.0;
    }

    if dst_x1 > dst.w as f64 {
        src_x1 -= (dst_x1 - dst.w as f64) / scaling_w;
        dst_x1 = dst.w as f64;
    }

    if dst_y0 < 0.0 {
        src_y0 -= dst_y0 / scaling_h;
        dst_y0 = 0.0;
    }

    if dst_y1 > dst.h as f64 {
        src_y1 -= (dst_y1 - dst.h as f64) / scaling_h;
        dst_y1 = dst.h as f64;
    }

    dst_x0 += dst_clip_rect.x as f64;
    dst_x1 += dst_clip_rect.x as f64;
    dst_y0 += dst_clip_rect.y as f64;
    dst_y1 += dst_clip_rect.y as f64;

    let final_src = Rect::new(
        src_x0.round() as i32,
        src_y0.round() as i32,
        (src_x1 - src_x0).round() as u32,
        (src_y1 - src_y0).round() as u32,
    );

    let final_dst = Rect::new(
        dst_x0.round() as i32,
        dst_y0.round() as i32,
        (dst_x1 - dst_x0).round() as u32,
        (dst_y1 - dst_y0).round() as u32,
    );

    let final_src =
        Rect::new(0, 0, src_rect.w as u32, src_rect.h as u32).intersection(final_src)?;
    let final_dst = dst_clip_rect.intersection(final_dst)?;

    Some((final_src, final_dst))
}
