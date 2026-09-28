use std::fs;
use std::path::Path;

use l2dat_editor::editor::{Editor, Options, SOURCE_KEY};

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
        editor.save(&document, &destination, SOURCE_KEY).unwrap();
        assert_eq!(fs::read(destination).unwrap(), expected, "{name}");
    }
}
