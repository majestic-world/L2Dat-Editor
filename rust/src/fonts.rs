use eframe::egui;

#[cfg(target_os = "windows")]
use std::{env, fs, path::PathBuf};

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    #[cfg(target_os = "windows")]
    if let Some(windows) = env::var_os("WINDIR") {
        let directory = PathBuf::from(windows).join("Fonts");
        // Use the installed Windows fonts rather than redistributing licensed assets.
        for (name, primary) in [
            ("segoeui.ttf", true),
            ("malgun.ttf", false),
            ("msyh.ttc", false),
        ] {
            if let Ok(bytes) = fs::read(directory.join(name)) {
                fonts
                    .font_data
                    .insert(name.to_owned(), egui::FontData::from_owned(bytes).into());
                let proportional = fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default();
                if primary {
                    proportional.insert(0, name.to_owned());
                } else {
                    proportional.push(name.to_owned());
                }
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push(name.to_owned());
            }
        }
    }
    ctx.set_fonts(fonts);
}
