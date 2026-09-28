use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, FontId, RichText};
use l2dat_editor::editor::{BatchKind, TextEncoding};

use super::EditorApp;
use crate::icons::{self, Icon};
use crate::theme;

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).size(12.0).strong().color(theme::MUTED));
    ui.add_space(6.0);
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .inner_margin(egui::Margin::symmetric(16, 8))
}

impl EditorApp {
    fn toggle_goto(&mut self, ctx: &egui::Context) {
        self.goto_visible = !self.goto_visible;
        if self.goto_visible {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("goto_line")));
        }
    }

    fn batch_actions(&mut self, ui: &mut egui::Ui) {
        for (icon, label, kind) in [
            (Icon::Unpack, "Extrair arquivos", BatchKind::Unpack),
            (Icon::Pack, "Empacotar arquivos", BatchKind::Pack),
            (Icon::Recrypt, "Trocar criptografia", BatchKind::Recrypt),
        ] {
            if icons::button(ui, icon, label).clicked() {
                self.batch_dialog = Some(kind);
                ui.close_menu();
            }
        }
    }

    pub(super) fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("application_header")
            .frame(panel_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.set_height(28.0);
                    ui.add(icons::image(ui.ctx(), Icon::Layers, 26.0).tint(theme::ACCENT));
                    ui.add_space(6.0);
                    ui.label(RichText::new("L2 DAT STUDIO").strong().size(19.0));
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_enabled_ui(self.job.is_none(), |ui| {
                        ui.menu_button("Arquivo", |ui| {
                            if icons::button(ui, Icon::Folder, "Abrir arquivo…    Ctrl+O").clicked()
                            {
                                ui.close_menu();
                                self.choose_open(ctx);
                            }
                            ui.add_enabled_ui(self.document.is_some(), |ui| {
                                if icons::button(ui, Icon::Save, "Salvar    Ctrl+S").clicked() {
                                    ui.close_menu();
                                    self.save_document(ctx, false);
                                }
                                if icons::button(ui, Icon::SaveAs, "Salvar como…").clicked() {
                                    ui.close_menu();
                                    self.save_document(ctx, true);
                                }
                                if icons::button(ui, Icon::Export, "Exportar TXT…").clicked() {
                                    ui.close_menu();
                                    self.export(ctx);
                                }
                            });
                        });
                        ui.add_enabled_ui(self.document.is_some(), |ui| {
                            ui.menu_button("Editar", |ui| {
                                if icons::button(ui, Icon::Search, "Buscar / substituir    Ctrl+F")
                                    .clicked()
                                {
                                    ui.close_menu();
                                    self.open_search();
                                }
                                if icons::button(ui, Icon::GoTo, "Ir à linha    Ctrl+G").clicked()
                                {
                                    ui.close_menu();
                                    self.toggle_goto(ctx);
                                }
                            });
                        });
                        ui.menu_button("Ferramentas", |ui| self.batch_actions(ui));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::Frame::new()
                            .stroke(egui::Stroke::new(1.0_f32, theme::ACCENT))
                            .corner_radius(3.0)
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(concat!(
                                        "L2DAT Studio v",
                                        env!("CARGO_PKG_VERSION"),
                                        " - By Mk"
                                    ))
                                    .size(11.0)
                                    .color(theme::ACCENT),
                                );
                            });
                    });
                });
            });
        egui::TopBottomPanel::top("actions")
            .frame(panel_frame())
            .show(ctx, |ui| {
                ui.add_enabled_ui(self.job.is_none(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.set_min_height(28.0);
                        if icons::button(ui, Icon::Folder, "Abrir arquivo")
                            .on_hover_text("Abrir arquivo… (Ctrl+O)")
                            .clicked()
                        {
                            self.choose_open(ctx);
                        }
                        ui.add_enabled_ui(self.document.is_some(), |ui| {
                            if icons::button(ui, Icon::Save, "Salvar")
                                .on_hover_text("Salvar (Ctrl+S)")
                                .clicked()
                            {
                                self.save_document(ctx, false);
                            }
                            if icons::button(ui, Icon::SaveAs, "Salvar como").clicked() {
                                self.save_document(ctx, true);
                            }
                            ui.separator();
                            if icons::button(ui, Icon::Export, "Exportar TXT").clicked() {
                                self.export(ctx);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if icons::button(ui, Icon::GoTo, "Ir à linha")
                                        .on_hover_text("Ir à linha (Ctrl+G)")
                                        .clicked()
                                    {
                                        self.toggle_goto(ctx);
                                    }
                                    ui.separator();
                                    if icons::button(ui, Icon::Search, "Buscar")
                                        .on_hover_text("Buscar / substituir (Ctrl+F)")
                                        .clicked()
                                    {
                                        self.open_search();
                                    }
                                },
                            );
                        });
                    });
                });
            });
    }

    pub(super) fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("workspace")
            .resizable(true)
            .default_width(250.0)
            .width_range(220.0..=380.0)
            .frame(panel_frame())
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Área de trabalho").size(15.0).strong());
                    section(ui, "Arquivos recentes");
                    let mut selected = None;
                    ui.add_enabled_ui(self.job.is_none(), |ui| {
                        if self.settings.recent.is_empty() {
                            ui.label(
                                RichText::new("Os arquivos abertos aparecem aqui.")
                                    .color(theme::MUTED)
                                    .size(12.0),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("recent_files")
                            .max_height(180.0)
                            .show(ui, |ui| {
                                for path in &self.settings.recent {
                                    let active =
                                        self.document.as_ref().is_some_and(|doc| doc.path == *path);
                                    let name =
                                        path.file_name().unwrap_or_default().to_string_lossy();
                                    let response = ui
                                        .add_sized(
                                            [ui.available_width(), 32.0],
                                            egui::Button::image_and_text(
                                                icons::image(ui.ctx(), Icon::File, 20.0).tint(
                                                    if active {
                                                        theme::ACCENT
                                                    } else {
                                                        theme::MUTED
                                                    },
                                                ),
                                                name.as_ref(),
                                            )
                                            .frame(active)
                                            .fill(
                                                if active {
                                                    theme::RAISED
                                                } else {
                                                    Color32::TRANSPARENT
                                                },
                                            ),
                                        )
                                        .on_hover_text(path.display().to_string());
                                    if active {
                                        ui.painter().line_segment(
                                            [response.rect.left_top(), response.rect.left_bottom()],
                                            egui::Stroke::new(3.0_f32, theme::ACCENT),
                                        );
                                    }
                                    if response.clicked() {
                                        selected = Some(path.clone());
                                    }
                                }
                            });
                    });
                    if let Some(path) = selected {
                        self.open(ctx, path);
                    }
                    ui.add_space(14.0);
                    ui.separator();
                    section(ui, "Configuração");
                    ui.add_enabled_ui(self.job.is_none(), |ui| {
                        ui.visuals_mut().widgets.inactive.bg_stroke =
                            egui::Stroke::new(1.0_f32, theme::BORDER);
                        ui.visuals_mut().widgets.inactive.weak_bg_fill = theme::RAISED;
                        let mut changed = false;
                        ui.label(RichText::new("Crônica").color(theme::MUTED));
                        egui::ComboBox::from_id_salt("chronicle")
                            .selected_text(&self.settings.chronicle)
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for name in &self.chronicles {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.settings.chronicle,
                                            name.clone(),
                                            name,
                                        )
                                        .changed();
                                }
                            });
                        ui.add_space(6.0);
                        ui.label(RichText::new("Gravação").color(theme::MUTED));
                        egui::ComboBox::from_id_salt("encryption")
                            .selected_text(&self.settings.encryption)
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for name in &self.encryptions {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.settings.encryption,
                                            name.clone(),
                                            name,
                                        )
                                        .changed();
                                }
                            });
                        ui.add_space(10.0);
                        changed |= ui
                            .checkbox(&mut self.settings.formatter, "Formatadores")
                            .changed();
                        changed |= ui.checkbox(&mut self.settings.enums, "Enums").changed();
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Crônica, formatadores e enums aplicados na próxima abertura.",
                            )
                            .size(11.0)
                            .color(theme::MUTED),
                        );
                        if changed {
                            self.save_settings();
                        }
                    });
                    ui.add_space(14.0);
                    ui.separator();
                    section(ui, "Operações em lote");
                    ui.add_enabled_ui(self.job.is_none(), |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        self.batch_actions(ui);
                    });
                });
            });
    }

    pub(super) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .frame(panel_frame().inner_margin(egui::Margin::symmetric(16, 4)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.job.is_some() {
                        ui.add(egui::Spinner::new().size(12.0));
                        if let Some(progress) = self.progress {
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .desired_width(80.0)
                                    .desired_height(4.0)
                                    .fill(theme::ACCENT),
                            )
                            .on_hover_text(if self.batch_running {
                                "Progresso por arquivos concluídos"
                            } else {
                                "Etapas concluídas: codificação, criptografia, gravação e reabertura"
                            });
                        }
                        if self.batch_running && ui.button("Cancelar lote").clicked() {
                            self.cancel.store(true, Ordering::Relaxed);
                            self.activity =
                                "Cancelamento solicitado; aguardando o arquivo atual".into();
                        }
                    } else {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12.0, 16.0), egui::Sense::hover());
                        ui.painter().circle_filled(
                            rect.center(),
                            4.0,
                            if self.activity == "Pronto" {
                                theme::SUCCESS
                            } else {
                                theme::ERROR
                            },
                        );
                    }
                    ui.add(egui::Label::new(RichText::new(&self.activity).size(12.0)).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(doc) = &self.document {
                            ui.label(
                                RichText::new(&doc.options.chronicle)
                                    .size(11.0)
                                    .color(theme::MUTED),
                            );
                            ui.separator();
                            ui.label(
                                RichText::new(format!(
                                    "{} linhas",
                                    self.text_editor.state.line_count()
                                ))
                                .size(11.0)
                                .color(theme::MUTED),
                            );
                            ui.separator();
                            let encoding = match doc.encoding {
                                TextEncoding::Utf8 => "UTF-8",
                                TextEncoding::Utf8Bom => "UTF-8 BOM",
                                TextEncoding::Utf16Le => "UTF-16 LE",
                                TextEncoding::Utf16Be => "UTF-16 BE",
                            };
                            ui.label(RichText::new(encoding).size(11.0).color(theme::MUTED));
                            if self.dirty {
                                ui.separator();
                                ui.label(
                                    RichText::new("Não salvo").size(11.0).color(theme::ACCENT),
                                );
                            }
                        }
                    });
                });
            });
    }

    pub(super) fn output_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(150.0)
            .min_height(70.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Saída").strong());
                    ui.label(
                        RichText::new(format!("{} registros", self.messages.len()))
                            .size(11.0)
                            .color(theme::MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::button(ui, Icon::Trash, "Limpar").clicked() {
                            self.messages.clear();
                        }
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (error, message) in &self.messages {
                            ui.horizontal_top(|ui| {
                                ui.label(
                                    RichText::new(if *error { "!" } else { ">" })
                                        .font(FontId::monospace(12.0))
                                        .color(if *error { theme::ERROR } else { theme::SUCCESS }),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(message).font(FontId::monospace(12.0)).color(
                                            if *error { theme::ERROR } else { theme::MUTED },
                                        ),
                                    )
                                    .wrap(),
                                );
                            });
                        }
                    });
            });
    }

    pub(super) fn empty_editor(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.22).max(24.0));
            ui.add(icons::image(ui.ctx(), Icon::Layers, 48.0).tint(theme::ACCENT));
            ui.add_space(18.0);
            ui.label(
                RichText::new("Seu próximo arquivo começa aqui.")
                    .size(23.0)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Selecione a crônica na lateral e abra um arquivo do cliente.")
                    .color(theme::MUTED),
            );
            ui.add_space(20.0);
            ui.add_enabled_ui(self.job.is_none(), |ui| {
                if ui
                    .add(
                        egui::Button::image_and_text(
                            icons::image(ui.ctx(), Icon::Folder, 20.0).tint(theme::BG),
                            RichText::new("Abrir arquivo").color(theme::BG).strong(),
                        )
                        .fill(theme::ACCENT)
                        .min_size(egui::vec2(150.0, 36.0)),
                    )
                    .clicked()
                {
                    self.choose_open(ctx);
                }
            });
            ui.add_space(12.0);
            ui.label(
                RichText::new("DAT, INI, HTM ou TXT  •  Ctrl+O")
                    .size(12.0)
                    .color(theme::MUTED),
            );
            ui.label(
                RichText::new("Você também pode arrastar um arquivo para esta janela.")
                    .size(12.0)
                    .color(theme::MUTED),
            );
        });
    }
}
