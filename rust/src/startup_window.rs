use eframe::egui::{Rect, Vec2, ViewportBuilder, pos2, vec2};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForSystem};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SPI_GETWORKAREA, SystemParametersInfoW, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW,
};

pub fn fit_to_work_area(viewport: ViewportBuilder) -> ViewportBuilder {
    let mut work = RECT::default();
    let mut frame = RECT::default();
    // eframe invokes this hook after winit enables per-monitor DPI awareness.
    // SPI_GETWORKAREA and eframe's initial scale both refer to the primary monitor.
    // SAFETY: both RECT pointers are valid writable storage for the synchronous calls.
    let dpi = unsafe {
        let dpi = GetDpiForSystem();
        if dpi == 0
            || SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut work).cast(), 0) == 0
            || AdjustWindowRectExForDpi(&mut frame, WS_OVERLAPPEDWINDOW, 0, WS_EX_APPWINDOW, dpi)
                == 0
        {
            return viewport;
        }
        dpi
    };
    let scale = dpi as f32 / 96.0;
    let work = Rect::from_min_max(
        pos2(work.left as f32 / scale, work.top as f32 / scale),
        pos2(work.right as f32 / scale, work.bottom as f32 / scale),
    );
    let frame = vec2(
        (frame.right - frame.left) as f32 / scale,
        (frame.bottom - frame.top) as f32 / scale,
    );
    fit(viewport, work, frame)
}

fn fit(viewport: ViewportBuilder, work: Rect, frame: Vec2) -> ViewportBuilder {
    let size = viewport
        .inner_size
        .unwrap_or(vec2(1100.0, 700.0))
        .min((work.size() * 0.85 - frame).max(Vec2::splat(1.0)));
    let minimum = viewport.min_inner_size.unwrap_or(Vec2::ZERO).min(size);
    viewport
        .with_inner_size(size)
        .with_min_inner_size(minimum)
        .with_position(work.center() - (size + frame) * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_fits_and_centers_outer_window_at_different_scales() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            // Include a top/left taskbar and a work area smaller than the old minimum.
            let work = Rect::from_min_max(pos2(48.0, 40.0) / scale, pos2(1366.0, 728.0) / scale);
            let frame = vec2(16.0, 39.0);
            let viewport = fit(
                ViewportBuilder::default()
                    .with_inner_size([1100.0, 700.0])
                    .with_min_inner_size([900.0, 600.0]),
                work,
                frame,
            );
            let size = viewport.inner_size.unwrap();
            let outer = Rect::from_min_size(viewport.position.unwrap(), size + frame);
            assert!(
                work.contains_rect(outer),
                "Window must fit at scale {scale}"
            );
            assert!(outer.center().distance(work.center()) < 0.01);
            let minimum = viewport.min_inner_size.unwrap();
            assert!(minimum.x <= size.x && minimum.y <= size.y);
            assert!(outer.width() <= work.width() * 0.85 + 0.01);
            assert!(outer.height() <= work.height() * 0.85 + 0.01);
        }
    }
}
