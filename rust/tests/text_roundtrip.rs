use std::fs;
use std::path::Path;

use l2dat_editor::editor::{Editor, Options, SOURCE_KEY, SaveStage};

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
            .save(&document, &destination, SOURCE_KEY, |stage| {
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
            .save(&document, &destination, SOURCE_KEY, |stage| {
                written |= stage == SaveStage::Written;
            })
            .is_err()
    );
    assert!(!written, "A failed atomic write must not report completion");
    assert_eq!(
        fs::read_to_string(source).unwrap(),
        "[Engine]\nName=Before\n"
    );
    assert!(destination.is_dir());
}
