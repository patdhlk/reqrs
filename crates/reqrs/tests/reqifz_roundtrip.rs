use std::io::{Cursor, Write};

use pretty_assertions::assert_eq;
use reqrs::{FormatMode, ReqIfzBundle};
use zip::write::{SimpleFileOptions, ZipWriter};

const MINIMAL_REQIF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd"/>
"#;

fn build_test_zip() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zw.start_file("main.reqif", opts).unwrap();
        zw.write_all(MINIMAL_REQIF.as_bytes()).unwrap();
        zw.start_file("img.png", opts).unwrap();
        zw.write_all(&[0u8, 1, 2, 3]).unwrap();
        zw.finish().unwrap();
    }
    buf
}

#[test]
fn read_then_write_preserves_entries() {
    let bytes = build_test_zip();
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqifz");
    let out_path = dir.path().join("out.reqifz");
    std::fs::write(&in_path, &bytes).unwrap();

    let bundle = ReqIfzBundle::read(&in_path).unwrap();
    assert_eq!(bundle.bundles.len(), 1);
    assert_eq!(bundle.attachments.len(), 1);
    assert_eq!(bundle.attachments[0].0, "img.png");
    assert_eq!(bundle.attachments[0].1, vec![0u8, 1, 2, 3]);
    bundle.write(&out_path, FormatMode::Passthrough).unwrap();

    let bundle2 = ReqIfzBundle::read(&out_path).unwrap();
    assert_eq!(bundle2.bundles.len(), 1);
    assert_eq!(bundle2.attachments.len(), 1);
    assert_eq!(bundle2.attachments[0].1, vec![0u8, 1, 2, 3]);
    assert_eq!(bundle2.bundles[0].0, "main.reqif");
}

#[test]
fn read_then_write_preserves_entry_order() {
    let mut buf = Vec::new();
    {
        let mut zw = ZipWriter::new(Cursor::new(&mut buf));
        let opts = SimpleFileOptions::default();
        zw.start_file("z.bin", opts).unwrap();
        zw.write_all(b"z").unwrap();
        zw.start_file("main.reqif", opts).unwrap();
        zw.write_all(MINIMAL_REQIF.as_bytes()).unwrap();
        zw.start_file("a.bin", opts).unwrap();
        zw.write_all(b"a").unwrap();
        zw.finish().unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let in_path = dir.path().join("in.reqifz");
    std::fs::write(&in_path, &buf).unwrap();
    let bundle = ReqIfzBundle::read(&in_path).unwrap();

    // bundles holds main.reqif only.
    assert_eq!(bundle.bundles.len(), 1);
    // attachments preserves z.bin → a.bin order.
    assert_eq!(
        bundle
            .attachments
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>(),
        vec!["z.bin", "a.bin"]
    );
}
