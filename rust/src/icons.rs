use eframe::egui;

use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Close,
    Layers,
}

pub fn image(icon: Icon, size: f32) -> egui::Image<'static> {
    let source = match icon {
        Icon::Folder => egui::include_image!("../assets/icons/folder.svg"),
        Icon::Save => egui::include_image!("../assets/icons/save.svg"),
        Icon::SaveAs => egui::include_image!("../assets/icons/save-as.svg"),
        Icon::Export => egui::include_image!("../assets/icons/export.svg"),
        Icon::Search => egui::include_image!("../assets/icons/search.svg"),
        Icon::GoTo => egui::include_image!("../assets/icons/go-to.svg"),
        Icon::File => egui::include_image!("../assets/icons/file.svg"),
        Icon::Unpack => egui::include_image!("../assets/icons/unpack.svg"),
        Icon::Pack => egui::include_image!("../assets/icons/pack.svg"),
        Icon::Recrypt => egui::include_image!("../assets/icons/recrypt.svg"),
        Icon::Trash => egui::include_image!("../assets/icons/trash.svg"),
        Icon::Close => egui::include_image!("../assets/icons/close.svg"),
        Icon::Layers => egui::include_image!("../assets/icons/layers.svg"),
    };
    egui::Image::new(source).fit_to_exact_size(egui::vec2(size, size))
}

pub fn button(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let tint = if icon == Icon::Layers {
        theme::ACCENT
    } else {
        theme::TEXT
    };
    ui.add(egui::Button::image_and_text(
        image(icon, 18.0).tint(tint),
        label,
    ))
}
