mod fonts;
mod highlight;
mod icons;
mod search;
mod text_state;
mod text_view;
mod theme;
mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use l2dat_editor::editor::{BatchKind, Editor, Options, SOURCE_KEY, locate_data};
use l2dat_editor::settings::Settings;

#[derive(Parser)]
#[command(
    version,
    about = "Native Lineage 2 DAT editor; no command opens the desktop UI"
)]
struct Cli {
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    chronicle: Option<String>,
    #[arg(long, global = true, default_value = SOURCE_KEY)]
    encryption: String,
    #[arg(long, global = true)]
    no_formatter: bool,
    #[arg(long, global = true)]
    no_enums: bool,
    #[arg(long)]
    open: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List the chronicles and encryption keys from the existing XML files.
    Catalog,
    /// Decode a DAT to UTF-8 text, or recursively unpack a directory.
    Unpack { input: PathBuf, output: PathBuf },
    /// Encode text to DAT, or recursively pack a directory. Choose --encryption explicitly.
    Pack { input: PathBuf, output: PathBuf },
    /// Change encryption without interpreting the binary schema.
    Recrypt { input: PathBuf, output: PathBuf },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        if std::env::args_os().len() == 1 {
            rfd::MessageDialog::new()
                .set_title("L2DAT Studio By Mk")
                .set_description(format!("{error:#}"))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let editor = Arc::new(Editor::load(locate_data(cli.data_dir)?)?);
    let chronicles = editor.catalog.chronicles();
    ensure!(
        !chronicles.is_empty(),
        "No chronicles found in the data directory"
    );
    let chronicle = cli
        .chronicle
        .clone()
        .unwrap_or_else(|| chronicles.last().unwrap().clone());
    ensure!(
        chronicles.contains(&chronicle),
        "Unknown chronicle: {chronicle}. Use catalog to list names."
    );
    let options = Options {
        chronicle,
        encryption: cli.encryption.clone(),
        formatter: !cli.no_formatter,
        enums: !cli.no_enums,
    };
    if let Some(command) = cli.command {
        let (kind, input, output) = match command {
            Command::Catalog => {
                for warning in editor.catalog.diagnostics() {
                    eprintln!("XML warning: {warning}");
                }
                println!(
                    "Data: {}\nChronicles ({}):",
                    editor.data_dir.display(),
                    chronicles.len()
                );
                for name in chronicles {
                    println!("  {name}");
                }
                println!("Encryption keys:");
                for name in editor.crypto.encrypt_names() {
                    println!("  {name}");
                }
                println!("  Plaintext");
                return Ok(());
            }
            Command::Unpack { input, output } => (BatchKind::Unpack, input, output),
            Command::Pack { input, output } => (BatchKind::Pack, input, output),
            Command::Recrypt { input, output } => (BatchKind::Recrypt, input, output),
        };
        if input.is_dir() {
            let report = editor.batch(
                kind,
                &input,
                &output,
                &options,
                &AtomicBool::new(false),
                |done, total, message| {
                    println!("[{done}/{total}] {message}");
                },
            )?;
            println!("{} succeeded; {} failed", report.succeeded, report.failed);
            ensure!(report.failed == 0, "Batch completed with errors");
        } else {
            ensure!(
                !output.exists(),
                "Output already exists: {}",
                output.display()
            );
            match kind {
                BatchKind::Unpack => editor.export(&editor.open(&input, &options)?, &output)?,
                BatchKind::Pack => {
                    ensure!(
                        options.encryption != SOURCE_KEY,
                        "Choose --encryption, for example v413_encdec or Plaintext"
                    );
                    editor.save(
                        &editor.open(&input, &options)?,
                        &output,
                        &options.encryption,
                        |_| {},
                    )?;
                }
                BatchKind::Recrypt => editor.recrypt(&input, &output, &options.encryption)?,
            }
            println!("Written {}", output.display());
        }
        return Ok(());
    }
    let (mut settings, warning) = match Settings::load() {
        Ok(settings) => (settings, None),
        Err(error) => (Settings::default(), Some(format!("{error:#}"))),
    };
    if cli.chronicle.is_some() || !chronicles.contains(&settings.chronicle) {
        settings.chronicle = options.chronicle;
    }
    if cli.encryption != SOURCE_KEY {
        settings.encryption = cli.encryption;
    }
    if cli.no_formatter {
        settings.formatter = false;
    }
    if cli.no_enums {
        settings.enums = false;
    }
    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_icon(icons::window_icon(if cfg!(target_os = "windows") {
                16
            } else {
                256
            }))
            .with_inner_size([
                settings.width.clamp(900.0, 3840.0),
                settings.height.clamp(600.0, 2160.0),
            ])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "L2DAT Studio By Mk",
        native,
        Box::new(move |cc| {
            Ok(Box::new(ui::EditorApp::new(
                cc, editor, settings, cli.open, warning,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Desktop UI: {e}"))
}
