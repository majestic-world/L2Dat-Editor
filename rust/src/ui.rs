use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use eframe::egui::{self, Color32, FontId, RichText, TextEdit};
use l2dat_editor::editor::{BatchKind, Document, Editor, PLAIN_KEY, SOURCE_KEY};
use l2dat_editor::settings::Settings;

use crate::fonts;
use crate::search::Search;

const INK: Color32 = Color32::from_rgb(38, 55, 70);
const EDITOR: Color32 = Color32::from_rgb(24, 35, 45);
const PAPER: Color32 = Color32::from_rgb(232, 237, 242);
const ACCENT: Color32 = Color32::from_rgb(47, 111, 138);
const ERROR: Color32 = Color32::from_rgb(183, 68, 68);

pub struct EditorApp {
    editor: Arc<Editor>,
    settings: Settings,
    chronicles: Vec<String>,
    encryptions: Vec<String>,
    document: Option<Document>,
    dirty: bool,
    line_numbers: String,
    line_count: usize,
    messages: Vec<(bool, String)>,
    job: Option<mpsc::Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    batch_running: bool,
    progress: f32,
    activity: String,
    query: String,
    replacement: String,
    search_visible: bool,
    search: Search,
    goto_visible: bool,
    goto_line: String,
    editor_event: Option<egui::Event>,
    request_paste: bool,
    context_selection: Option<(usize, usize)>,
    pending_selection: Option<(usize, usize)>,
    batch_dialog: Option<BatchKind>,
    batch_input: String,
    batch_output: String,
    startup_open: Option<PathBuf>,
}

enum Event {
    Opened(Document),
    Saved(Document),
    Exported(PathBuf),
    Progress(usize, usize, String),
    Finished(String),
    Failed(String),
}

impl EditorApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        editor: Arc<Editor>,
        settings: Settings,
        startup_open: Option<PathBuf>,
        warning: Option<String>,
    ) -> Self {
        fonts::install(&cc.egui_ctx);
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = PAPER;
        visuals.override_text_color = Some(INK);
        visuals.selection.bg_fill = ACCENT;
        visuals.selection.stroke.color = Color32::WHITE;
        cc.egui_ctx.set_visuals(visuals);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        cc.egui_ctx.set_style(style);
        let chronicles = editor.catalog.chronicles();
        let mut encryptions = vec![SOURCE_KEY.to_owned(), PLAIN_KEY.to_owned()];
        encryptions.extend(editor.crypto.encrypt_names());
        let mut app = Self {
            editor,
            settings,
            chronicles,
            encryptions,
            document: None,
            dirty: false,
            line_numbers: String::new(),
            line_count: 0,
            messages: Vec::new(),
            job: None,
            cancel: Arc::new(AtomicBool::new(false)),
            batch_running: false,
            progress: 0.0,
            activity: "Pronto".into(),
            query: String::new(),
            replacement: String::new(),
            search_visible: false,
            search: Search::default(),
            goto_visible: false,
            goto_line: "1".into(),
            editor_event: None,
            request_paste: false,
            context_selection: None,
            pending_selection: None,
            batch_dialog: None,
            batch_input: String::new(),
            batch_output: String::new(),
            startup_open,
        };
        app.log(
            false,
            format!("Estruturas: {}", app.editor.data_dir.display()),
        );
        let warnings = app.editor.catalog.diagnostics().to_vec();
        for warning in warnings {
            app.log(true, format!("XML: {warning}"));
        }
        if let Some(warning) = warning {
            app.log(true, warning);
        }
        app
    }

    fn log(&mut self, error: bool, text: String) {
        if self.messages.len() >= 1000 {
            self.messages.drain(..100);
        }
        self.messages.push((error, text));
    }

    fn save_settings(&mut self) {
        if let Err(error) = self.settings.save() {
            self.log(true, format!("Preferências: {error:#}"));
        }
    }

    fn confirm_discard(&self) -> bool {
        !self.dirty || rfd::MessageDialog::new()
            .set_title("Alterações não salvas")
            .set_description("Descartar as alterações deste arquivo? Use Salvar ou Salvar como para preservá-las.")
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo).show() == rfd::MessageDialogResult::Yes
    }

    fn start(
        &mut self,
        ctx: &egui::Context,
        activity: &str,
        work: impl FnOnce(mpsc::Sender<Event>, Arc<AtomicBool>) + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        self.job = Some(receiver);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.batch_running = false;
        self.activity = activity.to_owned();
        self.progress = 0.0;
        let cancel = self.cancel.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            work(sender, cancel);
            ctx.request_repaint();
        });
    }

    fn open(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.job.is_some() || !self.confirm_discard() {
            return;
        }
        let editor = self.editor.clone();
        let options = self.settings.options();
        self.start(ctx, "Abrindo arquivo", move |sender, _| {
            let event = match editor.open(&path, &options) {
                Ok(document) => Event::Opened(document),
                Err(error) => Event::Failed(format!("{}: {error:#}", path.display())),
            };
            let _ = sender.send(event);
        });
    }

    fn choose_open(&mut self, ctx: &egui::Context) {
        let mut dialog =
            rfd::FileDialog::new().add_filter("Lineage 2 / texto", &["dat", "ini", "htm", "txt"]);
        if let Some(path) = self.settings.recent.first().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(path);
        }
        if let Some(path) = dialog.pick_file() {
            self.open(ctx, path);
        }
    }

    fn save_document(&mut self, ctx: &egui::Context, save_as: bool) {
        let Some(document) = &self.document else {
            return;
        };
        if self.job.is_some() {
            return;
        }
        let output = if save_as || document.path.extension().is_some_and(|s| s == "txt") {
            let suggestion = document.path.with_extension(
                if document.structured || document.path.extension().is_some_and(|s| s == "txt") {
                    "dat"
                } else {
                    document
                        .path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("dat")
                },
            );
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(suggestion.file_name().unwrap_or_default().to_string_lossy())
                .set_directory(document.path.parent().unwrap_or(std::path::Path::new(".")))
                .save_file()
            else {
                return;
            };
            path
        } else {
            document.path.clone()
        };
        if document.path.extension().is_some_and(|s| s == "txt")
            && self.settings.encryption == SOURCE_KEY
        {
            self.log(true, "Selecione uma criptografia explícita para empacotar TXT (por exemplo, v413_encdec).".into());
            return;
        }
        let document = document.clone();
        let encryption = self.settings.encryption.clone();
        let editor = self.editor.clone();
        self.start(ctx, "Salvando arquivo", move |sender, _| {
            let result = editor.save(&document, &output, &encryption).and_then(|()| {
                editor.open(&output, &document.options).map_err(|error| {
                    anyhow::anyhow!("Arquivo gravado, mas a reabertura falhou: {error:#}")
                })
            });
            let _ = sender.send(match result {
                Ok(doc) => Event::Saved(doc),
                Err(error) => Event::Failed(format!("{}: {error:#}", output.display())),
            });
        });
    }

    fn export(&mut self, ctx: &egui::Context) {
        let Some(document) = &self.document else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(
                document
                    .path
                    .with_extension("txt")
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
            .add_filter("Texto UTF-8", &["txt"])
            .save_file()
        else {
            return;
        };
        let document = document.clone();
        let editor = self.editor.clone();
        self.start(ctx, "Exportando texto", move |sender, _| {
            let _ = sender.send(match editor.export(&document, &path) {
                Ok(()) => Event::Exported(path),
                Err(error) => Event::Failed(format!("{error:#}")),
            });
        });
    }

    fn receive(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = &self.job {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        for event in events {
            if matches!(&event, Event::Opened(_)) {
                egui::text_edit::TextEditState::default()
                    .store(ctx, egui::Id::new("document_editor"));
                self.pending_selection = None;
            }
            match event {
                Event::Opened(doc) | Event::Saved(doc) => {
                    self.log(
                        false,
                        format!(
                            "Arquivo pronto: {} ({})",
                            doc.path.display(),
                            doc.source_key.as_deref().unwrap_or(PLAIN_KEY)
                        ),
                    );
                    self.settings.remember(&doc.path);
                    self.document = Some(doc);
                    self.dirty = false;
                    self.refresh_search();
                    self.rebuild_lines();
                    self.job = None;
                    self.activity = "Pronto".into();
                    self.save_settings();
                }
                Event::Exported(path) => {
                    self.log(false, format!("Texto exportado: {}", path.display()));
                    self.job = None;
                    self.activity = "Pronto".into();
                }
                Event::Progress(done, total, text) => {
                    self.progress = done as f32 / total.max(1) as f32;
                    self.log(text.starts_with("ERROR"), text);
                }
                Event::Finished(text) => {
                    self.log(false, text);
                    self.job = None;
                    self.activity = "Pronto".into();
                }
                Event::Failed(error) => {
                    self.log(true, error);
                    self.job = None;
                    self.activity = "Falha — consulte o registro".into();
                }
            }
        }
        if disconnected && self.job.is_some() {
            self.job = None;
            self.activity = "Operação interrompida".into();
            self.log(
                true,
                "O processamento terminou sem retornar resultado.".into(),
            );
        }
    }

    fn rebuild_lines(&mut self) {
        let count = self
            .document
            .as_ref()
            .map(|d| d.text.bytes().filter(|b| *b == b'\n').count() + 1)
            .unwrap_or(0);
        if count == self.line_count {
            return;
        }
        self.line_count = count;
        self.line_numbers = (1..=count)
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
    }

    fn refresh_search(&mut self) {
        if let Some(doc) = &self.document {
            if let Err(error) = self.search.refresh(&doc.text, &self.query) {
                self.log(true, format!("Busca: {error}"));
            }
        }
    }

    fn find(&mut self, ctx: &egui::Context, forward: bool) {
        let Some(doc) = &self.document else {
            return;
        };
        let state = egui::text_edit::TextEditState::load(ctx, egui::Id::new("document_editor"));
        let (start, end) = state
            .and_then(|state| state.cursor.char_range())
            .map(|range| {
                (
                    range.primary.index.min(range.secondary.index),
                    range.primary.index.max(range.secondary.index),
                )
            })
            .unwrap_or((0, 0));
        let byte_index = |index| {
            doc.text
                .char_indices()
                .nth(index)
                .map_or(doc.text.len(), |(offset, _)| offset)
        };
        if let Some(found) = self
            .search
            .navigate(forward, byte_index(start)..byte_index(end))
        {
            self.pending_selection = Some((
                doc.text[..found.start].chars().count(),
                doc.text[..found.end].chars().count(),
            ));
        }
    }

    fn go_to_line(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        match self.goto_line.parse::<usize>() {
            Ok(line) if line > 0 && line <= self.line_count => {
                let position = doc
                    .text
                    .split_inclusive('\n')
                    .take(line - 1)
                    .map(|part| part.chars().count())
                    .sum();
                self.pending_selection = Some((position, position));
            }
            _ => self.log(
                true,
                format!("Informe uma linha entre 1 e {}.", self.line_count),
            ),
        }
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("L2 DAT").size(23.0).strong());
                ui.label(RichText::new("Editor de estruturas").color(ACCENT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Rust • desktop nativo");
                });
            });
            ui.add_enabled_ui(self.job.is_none(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Abrir…").clicked() {
                        self.choose_open(ctx);
                    }
                    ui.menu_button("Recentes", |ui| {
                        let mut selected = None;
                        for path in &self.settings.recent {
                            if ui.button(path.display().to_string()).clicked() {
                                selected = Some(path.clone());
                            }
                        }
                        if self.settings.recent.is_empty() {
                            ui.label("Nenhum arquivo recente");
                        }
                        if let Some(path) = selected {
                            ui.close_menu();
                            self.open(ctx, path);
                        }
                    });
                    ui.add_enabled_ui(self.document.is_some(), |ui| {
                        if ui.button("Salvar").clicked() {
                            self.save_document(ctx, false);
                        }
                        if ui.button("Salvar como…").clicked() {
                            self.save_document(ctx, true);
                        }
                        if ui.button("Exportar TXT…").clicked() {
                            self.export(ctx);
                        }
                        if ui.button("Buscar / substituir").clicked() {
                            self.search_visible = !self.search_visible;
                            if self.search_visible {
                                ctx.memory_mut(|m| m.request_focus(egui::Id::new("search_query")));
                            }
                        }
                        if ui.button("Ir à linha").clicked() {
                            self.goto_visible = !self.goto_visible;
                            if self.goto_visible {
                                ctx.memory_mut(|m| m.request_focus(egui::Id::new("goto_line")));
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Extrair lote…").clicked() {
                        self.batch_dialog = Some(BatchKind::Unpack);
                    }
                    if ui.button("Empacotar lote…").clicked() {
                        self.batch_dialog = Some(BatchKind::Pack);
                    }
                    if ui.button("Trocar criptografia…").clicked() {
                        self.batch_dialog = Some(BatchKind::Recrypt);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut changed = false;
                    ui.label("Crônica");
                    egui::ComboBox::from_id_salt("chronicle")
                        .selected_text(&self.settings.chronicle)
                        .width(275.0)
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
                    ui.label("Gravação");
                    egui::ComboBox::from_id_salt("encryption")
                        .selected_text(&self.settings.encryption)
                        .width(160.0)
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
                    changed |= ui
                        .checkbox(&mut self.settings.formatter, "Formatadores")
                        .changed();
                    changed |= ui.checkbox(&mut self.settings.enums, "Enums").changed();
                    if changed {
                        self.save_settings();
                    }
                });
            });
            if self.search_visible {
                ui.add_enabled_ui(self.document.is_some() && self.job.is_none(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Buscar");
                        let response = ui.add(
                            TextEdit::singleline(&mut self.query)
                                .id(egui::Id::new("search_query"))
                                .desired_width(150.0),
                        );
                        if response.changed() {
                            self.refresh_search();
                        }
                        if ui.button("Anterior").clicked() {
                            self.find(ctx, false);
                        }
                        if ui.button("Próxima").clicked()
                            || response.lost_focus()
                                && ui.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                                })
                        {
                            self.find(ctx, true);
                        }
                        ui.label(self.search.summary());
                        ui.label("Substituir por");
                        ui.add(TextEdit::singleline(&mut self.replacement).desired_width(150.0));
                        if ui.button("Substituir todas").clicked() {
                            if let Some(doc) = &mut self.document {
                                if let Some((text, count)) =
                                    self.search.replace_all(&doc.text, &self.replacement)
                                {
                                    doc.text = text;
                                    self.dirty = true;
                                    self.refresh_search();
                                    self.rebuild_lines();
                                    self.log(
                                        false,
                                        format!("{count} ocorrência(s) substituída(s)."),
                                    );
                                }
                            }
                        }
                    });
                });
            }
            if self.goto_visible {
                ui.add_enabled_ui(self.document.is_some() && self.job.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Linha");
                        let response = ui.add(
                            TextEdit::singleline(&mut self.goto_line)
                                .id(egui::Id::new("goto_line"))
                                .desired_width(70.0),
                        );
                        if ui.button("Ir").clicked()
                            || response.lost_focus()
                                && ui.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                                })
                        {
                            self.go_to_line();
                        }
                        if ui.small_button("Fechar").clicked() {
                            self.goto_visible = false;
                        }
                    });
                });
            }
        });
    }

    fn batch_window(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.batch_dialog else {
            return;
        };
        let title = match kind {
            BatchKind::Unpack => "Extrair DAT para texto",
            BatchKind::Pack => "Empacotar texto em DAT",
            BatchKind::Recrypt => "Trocar criptografia",
        };
        let mut open = true;
        let mut run = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    "Pastas separadas. Inclui subpastas; não sobrescreve arquivos existentes.",
                );
                ui.horizontal(|ui| {
                    ui.label("Entrada");
                    ui.add(TextEdit::singleline(&mut self.batch_input).desired_width(380.0));
                    if ui.button("Escolher…").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.batch_input = path.display().to_string();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Saída   ");
                    ui.add(TextEdit::singleline(&mut self.batch_output).desired_width(380.0));
                    if ui.button("Escolher…").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.batch_output = path.display().to_string();
                        }
                    }
                });
                ui.label(format!("Crônica: {}", self.settings.chronicle));
                if kind != BatchKind::Unpack {
                    ui.label(format!("Criptografia: {}", self.settings.encryption));
                    if self.settings.encryption == SOURCE_KEY {
                        ui.colored_label(
                            ERROR,
                            "Selecione uma chave explícita na barra principal.",
                        );
                    }
                }
                ui.label(
                    "Cancelar interrompe entre arquivos; a gravação em andamento é concluída.",
                );
                run = ui
                    .add_enabled(
                        !self.batch_input.is_empty()
                            && !self.batch_output.is_empty()
                            && (kind == BatchKind::Unpack
                                || self.settings.encryption != SOURCE_KEY),
                        egui::Button::new("Executar"),
                    )
                    .clicked();
            });
        if !open {
            self.batch_dialog = None;
        }
        if run {
            self.batch_dialog = None;
            let input = PathBuf::from(&self.batch_input);
            let output = PathBuf::from(&self.batch_output);
            let editor = self.editor.clone();
            let options = self.settings.options();
            self.start(ctx, title, move |sender, cancel| {
                let result = editor.batch(
                    kind,
                    &input,
                    &output,
                    &options,
                    &cancel,
                    |done, total, message| {
                        let _ = sender.send(Event::Progress(done, total, message));
                    },
                );
                let _ = sender.send(match result {
                    Ok(report) => Event::Finished(format!(
                        "{}: {} concluído(s), {} erro(s).",
                        if report.cancelled {
                            "Lote cancelado"
                        } else {
                            "Lote finalizado"
                        },
                        report.succeeded,
                        report.failed
                    )),
                    Err(error) => Event::Failed(format!("{error:#}")),
                });
            });
            self.batch_running = true;
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive(ctx);
        if let Some(path) = self.startup_open.take() {
            self.open(ctx, path);
        }
        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.job.is_some() || !self.confirm_discard() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                if self.job.is_some() {
                    self.log(
                        true,
                        "Aguarde o processamento ou cancele o lote antes de fechar.".into(),
                    );
                }
            } else {
                if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
                    self.settings.width = rect.width();
                    self.settings.height = rect.height();
                }
                self.save_settings();
            }
        }
        if self.job.is_none() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
                self.choose_open(ctx);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
                self.save_document(ctx, false);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
                self.search_visible = true;
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("search_query")));
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
                self.goto_visible = true;
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("goto_line")));
            }
            let dropped = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
            if let Some(path) = dropped {
                self.open(ctx, path);
            }
        }
        self.toolbar(ctx);
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.job.is_some() {
                    ui.spinner();
                    ui.add(egui::ProgressBar::new(self.progress).desired_width(180.0));
                    if self.batch_running && ui.button("Cancelar lote").clicked() {
                        self.cancel.store(true, Ordering::Relaxed);
                        self.activity =
                            "Cancelamento solicitado; aguardando o arquivo atual".into();
                    }
                }
                ui.label(&self.activity);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} linhas", self.line_count));
                    if self.dirty {
                        ui.label(
                            RichText::new("Alterações não salvas")
                                .color(Color32::from_rgb(140, 95, 36)),
                        );
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("log")
            .resizable(true)
            .default_height(135.0)
            .min_height(65.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Registro de operações").strong());
                    if ui.small_button("Limpar").clicked() {
                        self.messages.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for (error, message) in &self.messages {
                            ui.label(
                                RichText::new(message)
                                    .color(if *error { ERROR } else { INK })
                                    .font(FontId::monospace(12.0)),
                            );
                        }
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(event) = self.editor_event.take() { ui.input_mut(|input| input.events.push(event)); }
            if self.request_paste {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                self.request_paste = false;
            }
            let Some(doc) = &mut self.document else {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.heading("Arquivos do cliente, estruturas preservadas");
                    ui.label("Selecione a crônica e abra um DAT, INI, HTM ou TXT.");
                    ui.add_space(16.0);
                    if ui.add_enabled(self.job.is_none(), egui::Button::new("Abrir arquivo…")).clicked() { self.choose_open(ctx); }
                    ui.add_space(12.0);
                    ui.label("Também é possível arrastar um arquivo para esta janela.");
                });
                return;
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(doc.path.file_name().unwrap_or_default().to_string_lossy()).strong());
                ui.label(doc.path.parent().unwrap_or(std::path::Path::new(".")).display().to_string());
            });
            ui.label(RichText::new(format!("Leitura: {}  |  {}  |  opções de crônica/formatação aplicam-se na próxima abertura", doc.options.chronicle, doc.source_key.as_deref().unwrap_or(PLAIN_KEY))).size(11.0));
            let id = egui::Id::new("document_editor");
            let scroll_to_selection = self.pending_selection.is_some();
            if let Some((start, end)) = self.pending_selection.take() {
                let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(start), egui::text::CCursor::new(end))));
                state.store(ctx, id);
                ctx.memory_mut(|m| m.request_focus(id));
            }
            let mut changed = false;
            egui::Frame::new().fill(EDITOR).inner_margin(10.0).show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(Color32::from_rgb(222, 231, 236));
                ui.visuals_mut().extreme_bg_color = EDITOR;
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let font = FontId::monospace(14.0);
                        let gutter_width = ui.fonts(|fonts| fonts.glyph_width(&font, '0')) * self.line_count.max(1).ilog10().saturating_add(1) as f32 + 10.0;
                        let (gutter, _) = ui.allocate_exact_size(egui::vec2(gutter_width, 0.0), egui::Sense::hover());
                        let prior_selection = egui::text_edit::TextEditState::load(ctx, id)
                            .and_then(|state| state.cursor.char_range())
                            .map(|range| (range.primary.index, range.secondary.index));
                        let output = ui.add_enabled_ui(self.job.is_none(), |ui| {
                            TextEdit::multiline(&mut doc.text).id(id).font(font.clone()).code_editor().frame(false)
                                .desired_width(f32::INFINITY).desired_rows(25).show(ui)
                        }).inner;
                        if output.response.hovered() && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary)) {
                            self.context_selection = prior_selection;
                        }
                        changed = output.response.changed();
                        for (label, row) in self.line_numbers.lines().zip(&output.galley.rows) {
                            let y = output.galley_pos.y + row.rect.top();
                            if y + row.height() >= ui.clip_rect().top() && y <= ui.clip_rect().bottom() {
                                ui.painter().text(egui::pos2(gutter.right() - 4.0, y), egui::Align2::RIGHT_TOP, label, font.clone(), Color32::from_rgb(117, 144, 159));
                            }
                        }
                        if scroll_to_selection {
                            if let Some(cursor) = output.cursor_range {
                                let rect = output.galley.pos_from_cursor(&cursor.primary).translate(output.galley_pos.to_vec2());
                                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                            }
                        }
                        let response = output.response;
                        response.context_menu(|ui| {
                            ui.add_enabled_ui(self.job.is_none(), |ui| {
                                for (label, event) in [
                                    ("Copiar  Ctrl+C", egui::Event::Copy),
                                    ("Recortar  Ctrl+X", egui::Event::Cut),
                                ] {
                                    if ui.button(label).clicked() {
                                        ctx.memory_mut(|memory| memory.request_focus(id));
                                        self.pending_selection = self.context_selection;
                                        self.editor_event = Some(event);
                                        ui.close_menu();
                                    }
                                }
                                if ui.button("Colar  Ctrl+V").clicked() {
                                    self.pending_selection = self.context_selection;
                                    self.request_paste = true;
                                    ctx.memory_mut(|memory| memory.request_focus(id));
                                    ui.close_menu();
                                }
                                if ui.button("Excluir").clicked() {
                                    self.pending_selection = self.context_selection;
                                    self.editor_event = Some(egui::Event::Key { key: egui::Key::Backspace, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
                                    ctx.memory_mut(|memory| memory.request_focus(id));
                                    ui.close_menu();
                                }
                                if ui.button("Selecionar tudo  Ctrl+A").clicked() {
                                    self.pending_selection = Some((0, doc.text.chars().count()));
                                    ui.close_menu();
                                }
                                if ui.button("Buscar  Ctrl+F").clicked() {
                                    self.search_visible = true;
                                    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("search_query")));
                                    ui.close_menu();
                                }
                                if ui.button("Ir à linha  Ctrl+G").clicked() {
                                    self.goto_visible = true;
                                    ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("goto_line")));
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                });
            });
            if changed {
                self.dirty = true;
                self.refresh_search();
                self.rebuild_lines();
            }
        });
        self.batch_window(ctx);
    }
}
