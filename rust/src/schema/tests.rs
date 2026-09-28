use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use roxmltree::Document;

use super::binary;
use super::{
    Catalog, CodecOptions, Descriptor, Enumeration, NameTable, compile_nodes, decode, encode,
    parse_nodes,
};

fn descriptor(xml: &str) -> Descriptor {
    let document = Document::parse(xml).unwrap();
    let root = document.root_element();
    let mut nodes = parse_nodes(root, false, &HashMap::new(), 0).unwrap();
    assert!(compile_nodes(&mut nodes).unwrap().is_empty());
    Descriptor {
        format: None,
        source: "test.dat".to_owned(),
        nodes: Arc::new(nodes),
        enums: Arc::new(HashMap::new()),
        raw: root.attribute("isRaw") == Some("true"),
        safe_package: root.attribute("isSafePackage") == Some("true"),
    }
}

#[test]
fn nested_cycles_and_wrappers_rebuild_binary_counts() {
    let desc = descriptor(
        r##"<file isSafePackage="true">
        <node name="count" reader="UINT"/>
        <for name="row" size="#count" hidden="false">
            <node name="id" reader="USHORT"/>
            <node name="value_count" reader="CNTR"/>
            <for name="values" size="#value_count">
                <node name="number" reader="INT"/>
                <node name="label" reader="ASCF"/>
            </for>
            <wrapper name="pair"><node name="x" reader="FLOAT"/><node name="y" reader="FLOAT"/></wrapper>
        </for>
    </file>"##,
    );
    let text = "row_begin\tid=12\tvalues={{7;[a;b=c]};{-8;[한국]}}\tpair={1.0;-2.5}\trow_end\r\nrow_begin\tid=13\tvalues={}\tpair={0.0;3.0}\trow_end";
    let mut names = NameTable::default();
    let options = CodecOptions::default();
    let bytes = encode(&desc, text, &options, &mut names).unwrap();
    assert_eq!(&bytes[..7], &[2, 0, 0, 0, 12, 0, 2]);
    assert_eq!(decode(&desc, &bytes, &options, &mut names).unwrap(), text);
    assert_eq!(
        encode(
            &desc,
            &decode(&desc, &bytes, &options, &mut names).unwrap(),
            &options,
            &mut names
        )
        .unwrap(),
        bytes
    );
}

#[test]
fn conditions_masks_aliases_defaults_and_enums_select_binary_fields() {
    let mut desc = descriptor(
        r##"<file>
        <node name="count" reader="UINT"/><for name="entry" old_name="old_entry" size="#count" hidden="false">
            <node name="kind" reader="UBYTE" enum_name="kind"/>
            <node name="flags" reader="UBYTE"/>
            <if name="first" param="#kind" val="first"><node name="value" old_name="old_value" reader="INT"/></if>
            <else><node name="value" reader="ASCF"/></else>
            <mask name="extra" param="#flags" val="2"><node name="extra" reader="SHORT" default_value="-5"/></mask>
        </for>
    </file>"##,
    );
    let mut enumeration = Enumeration::default();
    enumeration.names.insert(1, "first".to_owned());
    enumeration.indices.insert("first".to_owned(), 1);
    desc.enums = Arc::new(HashMap::from([("kind".to_owned(), enumeration)]));
    let text = "old_entry_begin\told_value=123\tflags=2\tkind=first\told_entry_end\r\nentry_begin\tkind=0\tflags=0\tvalue=[other]\tentry_end";
    let options = CodecOptions { enums: true };
    let mut names = NameTable::default();
    let bytes = encode(&desc, text, &options, &mut names).unwrap();
    assert_eq!(&bytes[..12], &[2, 0, 0, 0, 1, 2, 123, 0, 0, 0, 251, 255]);
    let decoded = decode(&desc, &bytes, &options, &mut names).unwrap();
    assert_eq!(
        decoded,
        "entry_begin\tkind=first\tflags=2\tvalue=123\textra=-5\tentry_end\r\nentry_begin\tkind=0\tflags=0\tvalue=[other]\tentry_end"
    );
    assert_eq!(
        encode(&desc, &decoded, &options, &mut names).unwrap(),
        bytes
    );
}

#[test]
fn shared_and_static_counts_reject_disagreeing_lists() {
    let desc = descriptor(
        r##"<file>
        <node name="count" reader="UBYTE"/>
        <for name="a" size="#count"><node name="a" reader="INT"/></for>
        <for name="b" size="#count" skipWriteSize="true"><node name="b" reader="INT"/></for>
        <for name="fixed" size="2"><node name="c" reader="SHORT"/></for>
    </file>"##,
    );
    let options = CodecOptions::default();
    let mut names = NameTable::default();
    let bytes = encode(&desc, "a={1;2}\tb={3;4}\tfixed={5;6}", &options, &mut names).unwrap();
    assert_eq!(bytes.len(), 21);
    assert_eq!(bytes[0], 2);
    assert!(encode(&desc, "a={1;2}\tb={3}\tfixed={5;6}", &options, &mut names).is_err());
    assert!(encode(&desc, "a={}\tb={}\tfixed={}", &options, &mut names).is_err());
}

#[test]
fn malformed_data_reports_offsets_without_trailing_byte_loss() {
    let desc = descriptor(
        r##"<file isSafePackage="true"><node name="count" reader="UINT"/><for name="row" size="#count" hidden="false"><node name="value" reader="INT"/></for></file>"##,
    );
    let options = CodecOptions::default();
    let mut names = NameTable::default();
    let mut bytes = encode(&desc, "row_begin\tvalue=7\trow_end", &options, &mut names).unwrap();
    bytes.push(99);
    let error = decode(&desc, &bytes, &options, &mut names)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("trailing bytes") && error.contains("test.dat") && error.contains("offset")
    );
    let error = format!(
        "{:#}",
        decode(&desc, &[1, 0, 0, 0, 9], &options, &mut names).unwrap_err()
    );
    assert!(error.contains("field value") && error.contains("0x4"));
    assert!(decode(&desc, &i32::MAX.to_le_bytes(), &options, &mut names).is_err());
    assert!(
        encode(
            &desc,
            "row_begin\tvalue=7\tunknown=8\trow_end",
            &options,
            &mut names
        )
        .is_err()
    );
}

#[test]
fn raw_string_and_modern_map_ids_preserve_their_contracts() {
    let options = CodecOptions::default();
    let raw = descriptor(
        r#"<file isRaw="true" isSafePackage="true"><node name="text" reader="ASCF"/></file>"#,
    );
    let mut names = NameTable::default();
    let raw_text = "Line one\r\n한국어 line two\n";
    let bytes = encode(&raw, raw_text, &options, &mut names).unwrap();
    assert_eq!(
        decode(&raw, &bytes, &options, &mut names).unwrap(),
        raw_text
    );
    let map = descriptor(
        r#"<file><node name="name" reader="MAP_INT_TRANSLATABLE"/><node name="valid" reader="UBYTE"/></file>"#,
    );
    let bytes = [1234i32.to_le_bytes().as_slice(), &[8]].concat();
    assert_eq!(
        decode(&map, &bytes, &options, &mut names).unwrap(),
        "\tname=[<StrID:1234>]\tvalid=8"
    );
    assert_eq!(
        encode(&map, "name=[<StrID:1234>]\tvalid=8", &options, &mut names).unwrap(),
        bytes
    );
    let mut empty = 0i32.to_le_bytes().to_vec();
    binary::string(&mut empty, "SafePackage", true).unwrap();
    let mut names = NameTable::from_bytes(&empty).unwrap();
    assert!(encode(&map, "name=[New.Name]\tvalid=999", &options, &mut names).is_err());
    assert!(!names.is_dirty());
    assert_eq!(names.to_bytes().unwrap(), empty);
    assert_eq!(
        encode(&map, "name=[New.Name]\tvalid=8", &options, &mut names).unwrap(),
        [0, 0, 0, 0, 8]
    );
    assert!(names.is_dirty());
}

#[test]
fn mapped_names_with_delimiters_preserve_ids_and_dictionary_on_save() {
    let desc = descriptor(
        r##"<file isSafePackage="true">
        <node name="count" reader="UINT"/>
        <for name="npc" size="#count" hidden="false">
            <node name="npc_id" reader="USHORT"/>
            <node name="sounds" reader="UINT"/>
            <for name="dialog_sound" size="#sounds">
                <node name="sound" reader="MAP_INT"/>
            </for>
        </for>
        </file>"##,
    );
    let deep_name = format!(
        "{}x{}",
        "[".repeat(super::MAX_DEPTH),
        "]".repeat(super::MAX_DEPTH)
    );
    let values = [
        "Npcdialog.guard_03;[Npcdialog.guard_05",
        "unexpected]suffix",
        "];[other",
        "<StrID:0>",
        deep_name.as_str(),
        "normal.Sound",
        "balanced [한국] ; {label}=value",
    ];
    let mut dictionary = (values.len() as i32).to_le_bytes().to_vec();
    for value in values {
        binary::unicode(&mut dictionary, value, false).unwrap();
    }
    binary::string(&mut dictionary, "SafePackage", true).unwrap();
    let mut names = NameTable::from_bytes(&dictionary).unwrap();
    let mut original = 1u32.to_le_bytes().to_vec();
    original.extend_from_slice(&42u16.to_le_bytes());
    original.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for index in 0..values.len() as i32 {
        original.extend_from_slice(&index.to_le_bytes());
    }
    binary::string(&mut original, "SafePackage", true).unwrap();
    let options = CodecOptions::default();
    let decoded = decode(&desc, &original, &options, &mut names).unwrap();
    let saved = encode(&desc, &decoded, &options, &mut names).unwrap();
    assert_eq!(saved, original, "Saving must preserve every name-table ID");
    assert_eq!(names.to_bytes().unwrap(), dictionary);
    assert!(!names.is_dirty());
    assert!(decoded.contains("[normal.Sound]"));
    assert!(decoded.contains("[balanced [한국] ; {label}=value]"));
    let edited = decoded.replace("npc_id=42", "npc_id=43");
    let saved = encode(&desc, &edited, &options, &mut names).unwrap();
    original[4..6].copy_from_slice(&43u16.to_le_bytes());
    assert_eq!(
        saved, original,
        "Editing another field must leave names intact"
    );
}

#[test]
fn supplied_catalog_resolves_modern_and_inherited_file_patterns() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../dist/data");
    let catalog = Catalog::load(&data).unwrap();
    assert!(
        catalog
            .chronicles()
            .iter()
            .any(|name| name == "Samurai (542)")
    );
    let desc = catalog
        .descriptor("Samurai (542)", "SYSSTRING-E.DAT")
        .unwrap();
    let text = "string_begin\tstringID=1\tstring=[Hello Rust]\tstring_end";
    let options = CodecOptions::default();
    let mut names = NameTable::default();
    let bytes = encode(&desc, text, &options, &mut names).unwrap();
    assert_eq!(decode(&desc, &bytes, &options, &mut names).unwrap(), text);
    assert!(
        catalog
            .diagnostics()
            .iter()
            .any(|message| message.contains("hairgrp") && message.contains("counter"))
    );
    assert!(
        catalog
            .descriptor("Kamael", "missing.dat")
            .unwrap_err()
            .to_string()
            .contains("missing parent ID 70")
    );
}
