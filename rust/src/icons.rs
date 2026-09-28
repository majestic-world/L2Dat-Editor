use std::collections::HashMap;

use eframe::egui;

use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Folder,
    Save,
    SaveAs,
    Export,
    Search,
    GoTo,
    File,
    Unpack,
    Pack,
    Recrypt,
    Trash,
    Layers,
}

#[derive(Clone, Default)]
struct IconTextures(HashMap<(Icon, u32), egui::TextureHandle>);

pub fn image(ctx: &egui::Context, icon: Icon, size: f32) -> egui::Image<'static> {
    let pixels_per_point = ctx.pixels_per_point();
    let pixels = (size * pixels_per_point).round().max(1.0) as u32;
    let key = (icon, pixels);
    let cache_id = egui::Id::new("icon_textures");
    // egui's URI texture cache ignores size hints. Keep distinct rasters for each
    // physical size, including simultaneous sizes and monitor/zoom changes.
    let cached = ctx.data_mut(|data| {
        data.get_temp_mut_or_default::<IconTextures>(cache_id)
            .0
            .get(&key)
            .map(egui::TextureHandle::id)
    });
    let texture_id = cached.unwrap_or_else(|| {
        let bytes: &[u8] = match icon {
            Icon::Folder => include_bytes!("../assets/icons/folder.svg"),
            Icon::Save => include_bytes!("../assets/icons/save.svg"),
            Icon::SaveAs => include_bytes!("../assets/icons/save-as.svg"),
            Icon::Export => include_bytes!("../assets/icons/export.svg"),
            Icon::Search => include_bytes!("../assets/icons/search.svg"),
            Icon::GoTo => include_bytes!("../assets/icons/go-to.svg"),
            Icon::File => include_bytes!("../assets/icons/file.svg"),
            Icon::Unpack => include_bytes!("../assets/icons/unpack.svg"),
            Icon::Pack => include_bytes!("../assets/icons/pack.svg"),
            Icon::Recrypt => include_bytes!("../assets/icons/recrypt.svg"),
            Icon::Trash => include_bytes!("../assets/icons/trash.svg"),
            Icon::Layers => include_bytes!("../assets/icons/layers.svg"),
        };
        let raster = egui_extras::image::load_svg_bytes_with_size(
            bytes,
            Some(egui::load::SizeHint::Size(pixels, pixels)),
        )
        .expect("Embedded icon SVG must be valid");
        let texture = ctx.load_texture(
            format!("{icon:?}-{pixels}px"),
            raster,
            // The SVG raster already has antialiasing. Sample it 1:1 without
            // adding bilinear blur when a layout starts at a fractional pixel.
            egui::TextureOptions::NEAREST,
        );
        let id = texture.id();
        ctx.data_mut(|data| {
            data.get_temp_mut_or_default::<IconTextures>(cache_id)
                .0
                .insert(key, texture);
        });
        id
    });
    egui::Image::new((texture_id, egui::Vec2::splat(pixels as f32)))
        .fit_to_exact_size(egui::Vec2::splat(pixels as f32 / pixels_per_point))
}

pub fn button(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let tint = if icon == Icon::Layers {
        theme::ACCENT
    } else {
        theme::TEXT
    };
    ui.add(egui::Button::image_and_text(
        image(ui.ctx(), icon, 20.0).tint(tint),
        label,
    ))
}

pub fn window_icon(pixels: u32) -> egui::IconData {
    let raster = egui_extras::image::load_svg_bytes_with_size(
        include_bytes!("../assets/app-icon.svg"),
        Some(egui::load::SizeHint::Size(pixels, pixels)),
    )
    .expect("Embedded application SVG must be valid");
    egui::IconData {
        rgba: raster
            .pixels
            .iter()
            .flat_map(|color| color.to_srgba_unmultiplied())
            .collect(),
        width: raster.width() as u32,
        height: raster.height() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_resolution_tracks_size_and_display_scale() {
        let ctx = egui::Context::default();
        for scale in [1.0, 1.25, 1.5, 2.0, 1.0] {
            ctx.set_zoom_factor(scale);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                for size in [26.0, 48.0, 20.0] {
                    let loaded = image(ctx, Icon::Layers, size)
                        .load_for_size(ctx, egui::Vec2::splat(size))
                        .expect("Embedded SVG must load");
                    assert_eq!(
                        loaded.size(),
                        Some(egui::Vec2::splat((size * scale).round())),
                        "Icon raster must match its physical display size"
                    );
                }
            });
        }
    }
}
