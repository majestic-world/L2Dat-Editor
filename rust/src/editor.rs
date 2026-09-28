use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail, ensure};
use encoding_rs::{UTF_16BE, UTF_16LE};
use tempfile::NamedTempFile;

use crate::crypto::CryptoCatalog;
use crate::format;
use crate::schema::{self, Catalog, CodecOptions, NameTable};

pub const SOURCE_KEY: &str = "Source";
pub const PLAIN_KEY: &str = "Plaintext";
const NAME_FILE: &str = "L2GameDataName.dat";

#[derive(Clone, Debug)]
pub struct Options {
    pub chronicle: String,
    pub encryption: String,
    pub formatter: bool,
    pub enums: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

/// Save phase boundaries. `Written` is emitted only after the output is persisted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveStage {
    Encoding,
    Encrypting,
    Writing,
    Written,
}

#[derive(Clone)]
pub struct Document {
    pub path: PathBuf,
    pub text: String,
    pub source_key: Option<String>,
    pub structured: bool,
    pub encoding: TextEncoding,
    pub crlf: bool,
    pub options: Options,
}

pub struct Editor {
    pub catalog: Catalog,
    pub crypto: CryptoCatalog,
    pub data_dir: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchKind {
    Unpack,
    Pack,
    Recrypt,
}

#[derive(Default, Debug)]
pub struct BatchReport {
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: bool,
}

impl Editor {
    pub fn load(data_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            catalog: Catalog::load(&data_dir)?,
            crypto: CryptoCatalog::load(&data_dir)?,
            data_dir,
        })
    }

    pub fn open(&self, path: &Path, options: &Options) -> Result<Document> {
        let bytes = fs::read(path).with_context(|| format!("Reading {}", path.display()))?;
        let filename = file_name(path)?;
        let decoded = self.crypto.decrypt(&bytes, filename)?;
        let extension = extension(path);
        let structured =
            extension == "dat" && (decoded.use_structure || decoded.key_name.is_none());
        let (text, encoding) = if structured {
            let desc = self.catalog.descriptor(&options.chronicle, filename)?;
            let mut names = if desc.uses_names() {
                self.load_names(path.parent().unwrap_or(Path::new(".")))?
            } else {
                NameTable::default()
            };
            let text = schema::decode(
                &desc,
                &decoded.bytes,
                &CodecOptions {
                    enums: options.enums,
                },
                &mut names,
            )
            .with_context(|| format!("Decoding {filename} using {}", options.chronicle))?;
            let text = if options.formatter {
                if let Some(name) = &desc.format {
                    format::decode_with_enums(name, &text, options.enums)?
                } else {
                    text
                }
            } else {
                text
            };
            (text, TextEncoding::Utf8)
        } else {
            decode_text(&decoded.bytes)?
        };
        Ok(Document {
            path: path.to_path_buf(),
            text: text.replace("\r\n", "\n"),
            source_key: decoded.key_name,
            structured,
            encoding,
            crlf: text.contains("\r\n"),
            options: options.clone(),
        })
    }

    /// Updates the document's saved metadata only after successful persistence.
    /// The session text is kept verbatim; saving never reopens the output.
    pub fn save(
        &self,
        document: &mut Document,
        output: &Path,
        encryption: &str,
        mut progress: impl FnMut(SaveStage),
    ) -> Result<()> {
        progress(SaveStage::Encoding);
        let key = self.resolve_key(encryption, document.source_key.as_deref())?;
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let structured =
            document.structured || extension(output) == "dat" && extension(&document.path) == "txt";
        let mut name_output = None;
        let payload = if structured {
            if let Some(key) = &key {
                ensure!(
                    self.crypto.encryption_uses_structure(key)?,
                    "Key {key} is configured for unstructured files; select v413_encdec, v413_encdec_bonux or Plaintext for a structured DAT"
                );
            }
            let desc = self
                .catalog
                .descriptor(&document.options.chronicle, file_name(output)?)?;
            let text = if document.options.formatter {
                if let Some(name) = &desc.format {
                    Cow::Owned(format::encode(name, &document.text)?)
                } else {
                    Cow::Borrowed(document.text.as_str())
                }
            } else {
                Cow::Borrowed(document.text.as_str())
            };
            let source_dir = document.path.parent().unwrap_or(Path::new("."));
            let names_dir = if parent.join(NAME_FILE).exists() {
                parent
            } else {
                source_dir
            };
            let mut names = if desc.uses_names() {
                self.load_names(names_dir)?
            } else {
                NameTable::default()
            };
            let payload = schema::encode(
                &desc,
                &text,
                &CodecOptions {
                    enums: document.options.enums,
                },
                &mut names,
            )?;
            if desc.uses_names()
                && !file_name(output)?.eq_ignore_ascii_case(NAME_FILE)
                && (names.is_dirty() || names_dir != parent && names_dir.join(NAME_FILE).exists())
            {
                name_output = Some(names.to_bytes()?);
            }
            payload
        } else {
            encode_text(&document.text, document.encoding, document.crlf)
        };
        progress(SaveStage::Encrypting);
        let name_output = name_output
            .map(|bytes| self.encrypt_payload(bytes, NAME_FILE, key.as_deref()))
            .transpose()?;
        let bytes = self.encrypt_payload(payload, file_name(output)?, key.as_deref())?;
        progress(SaveStage::Writing);
        // Install the dictionary first: extra unused names are harmless if the DAT write fails,
        // while a DAT referencing names not installed yet is not readable.
        if let Some(names) = name_output {
            atomic_write(&parent.join(NAME_FILE), &names)?;
        }
        atomic_write(output, &bytes)?;
        output.clone_into(&mut document.path);
        document.source_key = key;
        document.structured = structured;
        if structured {
            document.encoding = TextEncoding::Utf8;
        }
        progress(SaveStage::Written);
        Ok(())
    }

    pub fn export(&self, document: &Document, output: &Path) -> Result<()> {
        let same_file = output == document.path
            || output.exists()
                && document.path.exists()
                && output.canonicalize()? == document.path.canonicalize()?;
        ensure!(
            !same_file || extension(output) == "txt",
            "Export must not overwrite the source file"
        );
        atomic_write(output, document.text.as_bytes())
    }

    pub fn recrypt(&self, input: &Path, output: &Path, encryption: &str) -> Result<()> {
        ensure!(
            encryption != SOURCE_KEY,
            "Choose an explicit encryption key for re-encryption"
        );
        let decoded = self.crypto.decrypt(&fs::read(input)?, file_name(input)?)?;
        let key = self.resolve_key(encryption, decoded.key_name.as_deref())?;
        let bytes = self.encrypt_payload(decoded.bytes, file_name(output)?, key.as_deref())?;
        atomic_write(output, &bytes)
    }

    pub fn batch(
        &self,
        kind: BatchKind,
        input: &Path,
        output: &Path,
        options: &Options,
        cancel: &AtomicBool,
        mut progress: impl FnMut(usize, usize, String),
    ) -> Result<BatchReport> {
        ensure!(input.is_dir(), "Input must be a directory");
        fs::create_dir_all(output)?;
        let input = input.canonicalize()?;
        let output = output.canonicalize()?;
        ensure!(
            !output.starts_with(&input) && !input.starts_with(&output),
            "Input and output directories must be separate, not nested"
        );
        if kind != BatchKind::Unpack {
            ensure!(
                options.encryption != SOURCE_KEY,
                "Choose an explicit encryption key for a batch operation"
            );
        }
        let mut files = Vec::new();
        collect_files(&input, kind, &mut files)?;
        files.sort();
        ensure!(!files.is_empty(), "No matching files found");
        let mut report = BatchReport::default();
        for (index, path) in files.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            let relative = path.strip_prefix(&input)?;
            let mut target = output.join(relative);
            let result = (|| -> Result<()> {
                if kind == BatchKind::Pack && extension(path) == "txt" {
                    target.set_extension("dat");
                }
                match kind {
                    BatchKind::Unpack => {
                        let doc = self.open(path, options)?;
                        if doc.structured {
                            target.set_extension("txt");
                        }
                        ensure!(
                            !target.exists(),
                            "Output already exists: {}",
                            target.display()
                        );
                        let bytes = if doc.structured {
                            doc.text.as_bytes().to_vec()
                        } else {
                            encode_text(&doc.text, doc.encoding, doc.crlf)
                        };
                        atomic_write(&target, &bytes)?;
                        // Keep the shared dictionary with exported text for a later pack.
                        let dictionary = path.parent().unwrap().join(NAME_FILE);
                        let destination = target.parent().unwrap().join(NAME_FILE);
                        if doc.structured && dictionary.exists() && !destination.exists() {
                            atomic_write(&destination, &fs::read(dictionary)?)?;
                        }
                        Ok(())
                    }
                    BatchKind::Pack => {
                        ensure!(
                            !target.exists(),
                            "Output already exists: {}",
                            target.display()
                        );
                        let mut doc = if extension(path) == "dat"
                            && options.encryption != PLAIN_KEY
                            && !self.crypto.encryption_uses_structure(&options.encryption)?
                        {
                            let (text, encoding) = decode_text(&fs::read(path)?)?;
                            Document {
                                path: path.clone(),
                                crlf: text.contains("\r\n"),
                                text: text.replace("\r\n", "\n"),
                                encoding,
                                structured: false,
                                source_key: None,
                                options: options.clone(),
                            }
                        } else {
                            self.open(path, options)?
                        };
                        self.save(&mut doc, &target, &options.encryption, |_| {})
                    }
                    BatchKind::Recrypt => {
                        ensure!(
                            !target.exists(),
                            "Output already exists: {}",
                            target.display()
                        );
                        self.recrypt(path, &target, &options.encryption)
                    }
                }
            })();
            let message = match result {
                Ok(()) => {
                    report.succeeded += 1;
                    format!("OK {}", relative.display())
                }
                Err(error) => {
                    report.failed += 1;
                    format!("ERROR {}: {error:#}", relative.display())
                }
            };
            progress(index + 1, files.len(), message);
        }
        Ok(report)
    }

    fn load_names(&self, directory: &Path) -> Result<NameTable> {
        let path = directory.join(NAME_FILE);
        if !path.exists() {
            return Ok(NameTable::default());
        }
        let decoded = self
            .crypto
            .decrypt(&fs::read(&path)?, NAME_FILE)
            .with_context(|| format!("Reading dictionary {}", path.display()))?;
        NameTable::from_bytes(&decoded.bytes)
    }

    fn resolve_key(&self, selection: &str, source: Option<&str>) -> Result<Option<String>> {
        match selection {
            PLAIN_KEY => Ok(None),
            SOURCE_KEY => source
                .map(|key| self.crypto.encryption_for_source(key))
                .transpose(),
            explicit => Ok(Some(explicit.to_owned())),
        }
    }

    fn encrypt_payload(
        &self,
        bytes: Vec<u8>,
        filename: &str,
        key: Option<&str>,
    ) -> Result<Vec<u8>> {
        match key {
            Some(key) => self.crypto.encrypt(&bytes, filename, key),
            None => Ok(bytes),
        }
    }
}

pub fn locate_data(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit.or_else(|| std::env::var_os("L2DAT_DATA_DIR").map(PathBuf::from)) {
        ensure!(
            path.join("structure").is_dir(),
            "Data directory has no structure folder: {}",
            path.display()
        );
        return Ok(path);
    }
    let mut candidates = vec![
        PathBuf::from("dist/data"),
        PathBuf::from("../dist/data"),
        PathBuf::from("data"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("data"));
            candidates.push(parent.join("../../../dist/data"));
        }
    }
    candidates.into_iter().find(|p| p.join("structure").is_dir())
        .context("Data directory not found. Use --data-dir <directory containing structure, enums and config>")
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut file = NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Writing {}", path.display()))?;
    Ok(())
}

fn collect_files(directory: &Path, kind: BatchKind, result: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_files(&path, kind, result)?;
        } else if file_type.is_file() {
            let ext = extension(&path);
            let include = match kind {
                BatchKind::Pack => {
                    matches!(ext.as_str(), "txt" | "dat" | "ini" | "htm")
                        && !file_name(&path)?.eq_ignore_ascii_case(NAME_FILE)
                }
                BatchKind::Unpack => {
                    matches!(ext.as_str(), "dat" | "ini" | "htm")
                        && !file_name(&path)?.eq_ignore_ascii_case(NAME_FILE)
                }
                BatchKind::Recrypt => matches!(ext.as_str(), "dat" | "ini" | "htm"),
            };
            if include {
                result.push(path);
            }
        }
    }
    Ok(())
}

pub fn file_name(path: &Path) -> Result<&str> {
    path.file_name()
        .and_then(|s| s.to_str())
        .context("File name must be valid Unicode")
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn decode_text(bytes: &[u8]) -> Result<(String, TextEncoding)> {
    if let Some(body) = bytes.strip_prefix(&[0xff, 0xfe]) {
        let (text, errors) = UTF_16LE.decode_without_bom_handling(body);
        ensure!(!errors, "Invalid UTF-16LE text");
        Ok((text.into_owned(), TextEncoding::Utf16Le))
    } else if let Some(body) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let (text, errors) = UTF_16BE.decode_without_bom_handling(body);
        ensure!(!errors, "Invalid UTF-16BE text");
        Ok((text.into_owned(), TextEncoding::Utf16Be))
    } else {
        let (body, encoding) = if let Some(body) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
            (body, TextEncoding::Utf8Bom)
        } else {
            (bytes, TextEncoding::Utf8)
        };
        match std::str::from_utf8(body) {
            Ok(text) => Ok((text.to_owned(), encoding)),
            Err(_) => {
                bail!("Text is not UTF-8 or BOM-marked UTF-16; choose the correct DAT structure")
            }
        }
    }
}

fn encode_text(text: &str, encoding: TextEncoding, crlf: bool) -> Vec<u8> {
    match encoding {
        TextEncoding::Utf8 | TextEncoding::Utf8Bom => {
            let mut bytes = Vec::with_capacity(text.len() + 3);
            if matches!(encoding, TextEncoding::Utf8Bom) {
                bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
            }
            let mut previous_cr = false;
            for byte in text.bytes() {
                if crlf && byte == b'\n' && !previous_cr {
                    bytes.push(b'\r');
                }
                bytes.push(byte);
                previous_cr = byte == b'\r';
            }
            bytes
        }
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            let little = matches!(encoding, TextEncoding::Utf16Le);
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            let mut previous_cr = false;
            for unit in text.encode_utf16() {
                if crlf && unit == 10 && !previous_cr {
                    bytes.extend_from_slice(&if little {
                        13u16.to_le_bytes()
                    } else {
                        13u16.to_be_bytes()
                    });
                }
                bytes.extend_from_slice(&if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
                previous_cr = unit == 13;
            }
            bytes
        }
    }
}
