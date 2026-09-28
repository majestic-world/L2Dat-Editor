use std::fs;
use std::path::Path;

use l2dat_editor::editor::{Editor, Options, PLAIN_KEY, SOURCE_KEY, SaveStage};

#[test]
fn editing_text_preserves_original_bom_and_crlf() {
    let editor = Editor::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist/data")).unwrap();
    let options = Options {
        chronicle: "Samurai (542)".into(),
        encryption: SOURCE_KEY.into(),
        formatter: false,
        enums: false,
    };
    let directory = tempfile::tempdir().unwrap();
    let original = "[Engine]\r\nName=Before\r\n";
    let edited = "[Engine]\r\nName=After\r\n";
    let utf16 = |text: &str| {
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    };
    for (name, input, expected) in [
        (
            "plain.ini",
            original.as_bytes().to_vec(),
            edited.as_bytes().to_vec(),
        ),
        (
            "bom.ini",
            [b"\xef\xbb\xbf".as_slice(), original.as_bytes()].concat(),
            [b"\xef\xbb\xbf".as_slice(), edited.as_bytes()].concat(),
        ),
        ("unicode.htm", utf16(original), utf16(edited)),
    ] {
        let source = directory.path().join(name);
        let destination = directory.path().join("saved").join(name);
        fs::write(&source, input).unwrap();
        let mut document = editor.open(&source, &options).unwrap();
        document.text = document.text.replace("Before", "After");
        let mut written = false;
        editor
            .save(&mut document, &destination, SOURCE_KEY, |stage| {
                if stage == SaveStage::Written {
                    assert_eq!(fs::read(&destination).unwrap(), expected, "{name}");
                    written = true;
                }
            })
            .unwrap();
        assert!(written, "Successful persistence must notify completion");
        assert_eq!(fs::read(destination).unwrap(), expected, "{name}");
    }
}

#[test]
fn failed_persistence_does_not_report_written() {
    let editor = Editor::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist/data")).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.ini");
    let destination = directory.path().join("destination.ini");
    fs::write(&source, "[Engine]\nName=Before\n").unwrap();
    fs::create_dir(&destination).unwrap();
    let options = Options {
        chronicle: "Samurai (542)".into(),
        encryption: SOURCE_KEY.into(),
        formatter: false,
        enums: false,
    };
    let mut document = editor.open(&source, &options).unwrap();
    document.text = "[Engine]\nName=After\n".into();
    let mut written = false;
    assert!(
        editor
            .save(&mut document, &destination, SOURCE_KEY, |stage| {
                written |= stage == SaveStage::Written;
            })
            .is_err()
    );
    assert!(!written, "A failed atomic write must not report completion");
    assert_eq!(document.path, source);
    assert_eq!(document.source_key, None);
    assert_eq!(document.text, "[Engine]\nName=After\n");
    assert_eq!(
        fs::read_to_string(source).unwrap(),
        "[Engine]\nName=Before\n"
    );
    assert!(destination.is_dir());
}

#[test]
fn save_as_keeps_session_layout_and_updates_subsequent_source_saves() {
    let editor = Editor::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist/data")).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("sysstring-e.txt");
    let destination = directory.path().join("saved").join("sysstring-e.dat");
    let text = "\r\n \t\r\nstring_begin\tstringID=1\tstring=[Antes]\tstring_end\r\n\r\n";
    let mut input = vec![0xff, 0xfe];
    input.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    fs::write(&source, &input).unwrap();
    let options = Options {
        chronicle: "Scions of Destiny (C4)".into(),
        encryption: SOURCE_KEY.into(),
        formatter: false,
        enums: false,
    };
    let mut document = editor.open(&source, &options).unwrap();
    editor
        .save(&mut document, &destination, "v413_encdec_bonux", |_| {})
        .unwrap();
    assert_eq!(document.path, destination);
    assert_eq!(document.text, text.replace("\r\n", "\n"));

    // Source must now use the selected RSA key and encode records, not UTF-16 text.
    document.text = document.text.replace("Antes", "Depois");
    let saved_path = document.path.clone();
    editor
        .save(&mut document, &saved_path, SOURCE_KEY, |_| {})
        .unwrap();
    let reopened = editor.open(&destination, &options).unwrap();
    assert_eq!(reopened.source_key.as_deref(), Some("v413_encdec_bonux"));
    assert_eq!(
        reopened.text.trim_end_matches('\n'),
        "string_begin\tstringID=1\tstring=[Depois]\tstring_end"
    );
    assert_eq!(
        document.text,
        text.replace("\r\n", "\n").replace("Antes", "Depois")
    );

    // Switching to plaintext must also become the source for subsequent saves.
    editor
        .save(&mut document, &saved_path, PLAIN_KEY, |_| {})
        .unwrap();
    document.text = document.text.replace("Depois", "Final");
    editor
        .save(&mut document, &saved_path, SOURCE_KEY, |_| {})
        .unwrap();
    let reopened = editor.open(&destination, &options).unwrap();
    assert_eq!(reopened.source_key, None);
    assert_eq!(
        reopened.text.trim_end_matches('\n'),
        "string_begin\tstringID=1\tstring=[Final]\tstring_end"
    );
    assert_eq!(fs::read(source).unwrap(), input);
}
