mod chrome;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use eframe::egui::{self, Color32, RichText, TextEdit};
use l2dat_editor::editor::{BatchKind, Document, Editor, PLAIN_KEY, SOURCE_KEY, SaveStage};
use l2dat_editor::settings::Settings;

use crate::fonts;
use crate::icons::{self, Icon};
use crate::search::Search;
use crate::text_view::TextEditor;
use crate::theme;

const EDITOR: Color32 = theme::BG;
const ACCENT: Color32 = theme::ACCENT;
const ERROR: Color32 = theme::ERROR;

pub struct EditorApp {
    editor: Arc<Editor>,
    settings: Settings,
    chronicles: Vec<String>,
    encryptions: Vec<String>,
    document: Option<Document>,
    dirty: bool,
    messages: Vec<(bool, String)>,
    job: Option<mpsc::Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    batch_running: bool,
    progress: Option<f32>,
    activity: String,
    query: String,
    replacement: String,
    search_visible: bool,
    search_focus_requested: bool,
    search_feedback: Option<String>,
    search: Search,
    goto_visible: bool,
    goto_line: String,
    editor_event: Option<egui::Event>,
    request_paste: bool,
    batch_dialog: Option<BatchKind>,
    batch_input: String,
    batch_output: String,
    startup_open: Option<PathBuf>,
    text_editor: TextEditor,
    #[cfg(target_os = "windows")]
    native_icon_size: u32,
}

enum Event {
    Opened(Document),
    Saved(Document),
    Exported(PathBuf),
    Saving(SaveStage),
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
        theme::install(&cc.egui_ctx);
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
            messages: Vec::new(),
            job: None,
            cancel: Arc::new(AtomicBool::new(false)),
            batch_running: false,
            progress: None,
            activity: "Pronto".into(),
            query: String::new(),
            replacement: String::new(),
            search_visible: false,
            search_focus_requested: false,
            search_feedback: None,
            search: Search::default(),
            goto_visible: false,
            goto_line: "1".into(),
            editor_event: None,
            request_paste: false,
            batch_dialog: None,
            batch_input: String::new(),
            batch_output: String::new(),
            startup_open,
            text_editor: TextEditor::default(),
            #[cfg(target_os = "windows")]
            native_icon_size: 16,
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
        self.progress = None;
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
        let repaint = ctx.clone();
        self.start(ctx, "Salvando arquivo", move |sender, _| {
            let result = editor
                .save(&document, &output, &encryption, |stage| {
                    let _ = sender.send(Event::Saving(stage));
                    repaint.request_repaint();
                })
                .and_then(|()| {
                    editor.open(&output, &document.options).map_err(|error| {
                        anyhow::anyhow!("Arquivo gravado, mas a reabertura falhou: {error:#}")
                    })
                });
            let _ = sender.send(match result {
                Ok(doc) => Event::Saved(doc),
                Err(error) => Event::Failed(format!("{}: {error:#}", output.display())),
            });
        });
        self.progress = Some(0.0);
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

    fn receive(&mut self) {
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
            match &event {
                Event::Opened(doc) => self.text_editor.reset(&doc.text),
                Event::Saved(doc)
                    if self
                        .document
                        .as_ref()
                        .is_none_or(|current| current.text != doc.text) =>
                {
                    self.text_editor.reset(&doc.text);
                }
                _ => {}
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
                    self.job = None;
                    self.activity = "Pronto".into();
                    self.save_settings();
                }
                Event::Exported(path) => {
                    self.log(false, format!("Texto exportado: {}", path.display()));
                    self.job = None;
                    self.activity = "Pronto".into();
                }
                Event::Saving(stage) => {
                    let (progress, activity) = match stage {
                        SaveStage::Encoding => (0.0, "Codificando arquivo"),
                        SaveStage::Encrypting => (0.25, "Criptografando arquivo"),
                        SaveStage::Writing => (0.5, "Gravando arquivo"),
                        SaveStage::Written => (0.75, "Reabrindo arquivo"),
                    };
                    self.progress = Some(progress);
                    self.activity = activity.into();
                }
                Event::Progress(done, total, text) => {
                    self.progress = Some(done as f32 / total.max(1) as f32);
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

    fn refresh_search(&mut self) {
        if let Some(doc) = &self.document {
            if let Err(error) = self.search.refresh(&doc.text, &self.query) {
                self.log(true, format!("Busca: {error}"));
            }
        }
    }

    fn find(&mut self, forward: bool) {
        if self.document.is_none() {
            return;
        }
        if let Some(found) = self
            .search
            .navigate(forward, self.text_editor.state.selection.range())
        {
            self.text_editor.select_range(found, false);
        }
    }

    fn go_to_line(&mut self) {
        if self.document.is_none() {
            return;
        }
        let line_count = self.text_editor.state.line_count();
        match self.goto_line.parse::<usize>() {
            Ok(line) if line > 0 && line <= line_count => {
                let position = self.text_editor.state.line_start(line - 1);
                self.text_editor.select_range(position..position, true);
            }
            _ => self.log(true, format!("Informe uma linha entre 1 e {line_count}.")),
        }
    }

    fn open_search(&mut self) {
        self.search_visible = true;
        self.search_focus_requested = true;
        self.search_feedback = None;
    }

    fn search_modal(&mut self, ctx: &egui::Context) {
        if !self.search_visible {
            return;
        }
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("search_modal"))
            .frame(
                egui::Frame::popup(&ctx.style())
                    .fill(theme::SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
                    .inner_margin(20.0),
            )
            .show(ctx, |ui| {
                ui.set_width(500.0_f32.min((ctx.screen_rect().width() - 80.0).max(200.0)));
                ui.horizontal(|ui| {
                    ui.add(icons::image(ui.ctx(), Icon::Search, 20.0).tint(ACCENT));
                    ui.label(RichText::new("Buscar e substituir").size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = icons::button(ui, Icon::Close, "")
                            .on_hover_text("Fechar busca (Esc)")
                            .clicked();
                    });
                });
                ui.add_space(16.0);
                ui.add_enabled_ui(self.document.is_some() && self.job.is_none(), |ui| {
                    ui.label("Buscar");
                    let query = ui.add(
                        TextEdit::singleline(&mut self.query)
                            .id(egui::Id::new("search_query"))
                            .desired_width(f32::INFINITY),
                    );
                    if self.search_focus_requested {
                        query.request_focus();
                        self.search_focus_requested = false;
                    }
                    if query.changed() {
                        self.search_feedback = None;
                        self.refresh_search();
                    }
                    let enter = (query.has_focus() || query.lost_focus())
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Anterior").clicked() {
                            self.find(false);
                            query.request_focus();
                        }
                        if ui.button("Próxima").clicked() || enter {
                            self.find(true);
                            query.request_focus();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(self.search.summary()).color(theme::MUTED));
                        });
                    });
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label("Substituir por");
                    if ui
                        .add(
                            TextEdit::singleline(&mut self.replacement)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        self.search_feedback = None;
                    }
                    ui.add_space(8.0);
                    if ui.button("Substituir todas").clicked() {
                        if let Some(doc) = &mut self.document {
                            if let Some((text, count)) =
                                self.search.replace_all(&doc.text, &self.replacement)
                            {
                                let length = doc.text.len();
                                if self
                                    .text_editor
                                    .state
                                    .replace(&mut doc.text, 0..length, &text)
                                {
                                    self.dirty = true;
                                }
                                self.refresh_search();
                                let message = format!("{count} ocorrência(s) substituída(s).");
                                self.log(false, message.clone());
                                self.search_feedback = Some(message);
                            } else {
                                self.search_feedback =
                                    Some("Nenhuma ocorrência para substituir.".into());
                            }
                        }
                    }
                    if let Some(message) = &self.search_feedback {
                        ui.add(
                            egui::Label::new(RichText::new(message).color(theme::ACCENT)).wrap(),
                        );
                    }
                });
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Enter: próxima ocorrência · Esc: fechar")
                            .size(11.0)
                            .color(theme::MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close |= ui.button("Fechar").clicked();
                    });
                });
            });
        let dismiss = modal.should_close();
        if close || dismiss {
            self.search_visible = false;
            self.text_editor
                .select_range(self.text_editor.state.selection.range(), true);
        }
    }

    fn goto_bar(&mut self, ctx: &egui::Context) {
        if !self.goto_visible {
            return;
        }
        egui::TopBottomPanel::top("goto")
            .frame(egui::Frame::new().fill(theme::SURFACE).inner_margin(10.0))
            .show(ctx, |ui| {
                ui.add_enabled_ui(
                    self.document.is_some() && self.job.is_none() && !self.search_visible,
                    |ui| {
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
                    },
                );
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
                            "Selecione uma chave explícita na configuração lateral.",
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
        #[cfg(target_os = "windows")]
        {
            // winit uses the viewport icon for Windows' small title-bar icon.
            // Rasterize at its native DPI instead of shrinking a 256 px PNG.
            let pixels = (16.0 * ctx.native_pixels_per_point().unwrap_or(1.0)).round() as u32;
            if pixels != self.native_icon_size {
                ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(Arc::new(
                    icons::window_icon(pixels),
                ))));
                self.native_icon_size = pixels;
            }
        }
        self.receive();
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
        if self.job.is_none()
            && self.document.is_some()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F))
        {
            self.open_search();
        }
        if self.job.is_none() && !self.search_visible {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
                self.choose_open(ctx);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
                self.save_document(ctx, false);
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
        self.status_bar(ctx);
        self.sidebar(ctx);
        self.goto_bar(ctx);
        self.output_panel(ctx);
        let mut search_requested = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(EDITOR))
            .show(ctx, |ui| {
                if let Some(event) = self.editor_event.take() {
                    ui.input_mut(|input| input.events.push(event));
                }
                if self.request_paste {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    self.request_paste = false;
                }
                let Some(doc) = &mut self.document else {
                    self.empty_editor(ui, ctx);
                    return;
                };
                egui::Frame::new().fill(theme::SURFACE).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let tab = egui::Frame::new()
                            .fill(theme::RAISED)
                            .inner_margin(egui::Margin::symmetric(16, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(icons::image(ui.ctx(), Icon::File, 20.0).tint(ACCENT));
                                    ui.label(
                                        doc.path.file_name().unwrap_or_default().to_string_lossy(),
                                    );
                                    if self.dirty {
                                        ui.colored_label(ACCENT, "•")
                                            .on_hover_text("Alterações não salvas");
                                    }
                                });
                            })
                            .response;
                        ui.painter().line_segment(
                            [tab.rect.left_top(), tab.rect.right_top()],
                            egui::Stroke::new(2.0_f32, ACCENT),
                        );
                    });
                });
                egui::Frame::new()
                    .fill(theme::RAISED)
                    .inner_margin(egui::Margin::symmetric(16, 7))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(
                            egui::Label::new(
                                RichText::new(doc.path.display().to_string())
                                    .size(12.0)
                                    .color(theme::MUTED),
                            )
                            .truncate(),
                        )
                        .on_hover_text(format!(
                            "{}\nLeitura: {} / {}",
                            doc.path.display(),
                            doc.options.chronicle,
                            doc.source_key.as_deref().unwrap_or(PLAIN_KEY)
                        ));
                    });
                let id = egui::Id::new("document_editor");
                let changed = egui::Frame::new()
                    .fill(EDITOR)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        let response = self.text_editor.show(
                            ui,
                            &mut doc.text,
                            self.job.is_none() && !self.search_visible,
                        );
                        let changed = response.changed();
                        response.context_menu(|ui| {
                            ui.add_enabled_ui(self.job.is_none() && !self.search_visible, |ui| {
                                for (label, event) in [
                                    ("Copiar  Ctrl+C", egui::Event::Copy),
                                    ("Recortar  Ctrl+X", egui::Event::Cut),
                                ] {
                                    if ui.button(label).clicked() {
                                        ctx.memory_mut(|memory| memory.request_focus(id));
                                        self.editor_event = Some(event);
                                        ui.close_menu();
                                    }
                                }
                                if ui.button("Colar  Ctrl+V").clicked() {
                                    self.request_paste = true;
                                    ctx.memory_mut(|memory| memory.request_focus(id));
                                    ui.close_menu();
                                }
                                if ui.button("Excluir").clicked() {
                                    self.editor_event = Some(egui::Event::Key {
                                        key: egui::Key::Backspace,
                                        physical_key: None,
                                        pressed: true,
                                        repeat: false,
                                        modifiers: egui::Modifiers::NONE,
                                    });
                                    ctx.memory_mut(|memory| memory.request_focus(id));
                                    ui.close_menu();
                                }
                                if ui.button("Selecionar tudo  Ctrl+A").clicked() {
                                    self.text_editor.select_range(0..doc.text.len(), true);
                                    ui.close_menu();
                                }
                                if ui.button("Buscar  Ctrl+F").clicked() {
                                    search_requested = true;
                                    ui.close_menu();
                                }
                                if ui.button("Ir à linha  Ctrl+G").clicked() {
                                    self.goto_visible = true;
                                    ctx.memory_mut(|memory| {
                                        memory.request_focus(egui::Id::new("goto_line"))
                                    });
                                    ui.close_menu();
                                }
                            });
                        });
                        changed
                    })
                    .inner;
                if changed {
                    self.dirty = true;
                    self.refresh_search();
                }
            });
        if search_requested {
            self.open_search();
        }
        self.search_modal(ctx);
        self.batch_window(ctx);
    }
}
