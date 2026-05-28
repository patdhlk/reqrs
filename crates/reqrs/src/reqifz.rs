use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::error::ReqIfError;
use crate::model::ReqIfBundle;
use crate::parse::ReqIfParser;
use crate::unparse::{FormatMode, ReqIfUnparser};

/// A `.reqifz` archive — a ZIP container holding one or more `.reqif`/`.xml`
/// files plus arbitrary binary attachments.
///
/// Both [`bundles`](Self::bundles) and [`attachments`](Self::attachments)
/// retain insertion order so that round-tripping preserves the on-disk
/// layout *within* each Vec. Note that interleaving between bundles and
/// attachments in the source zip cannot be reconstructed: this mirrors the
/// Python `reqif` package's `ReqIFZBundle` structure.
#[derive(Debug)]
pub struct ReqIfzBundle {
    /// Parsed ReqIF documents, keyed by their original zip entry name,
    /// in zip-file order.
    pub bundles: Vec<(String, ReqIfBundle)>,
    /// Non-ReqIF entries (images, fonts, OLE objects, ...), keyed by their
    /// original zip entry name, in zip-file order.
    pub attachments: Vec<(String, Vec<u8>)>,
}

impl ReqIfzBundle {
    /// Open a `.reqifz` file, parse every `.reqif` / `.xml` entry into a
    /// [`ReqIfBundle`], and retain everything else as raw bytes.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, ReqIfError> {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(file)?;
        let mut bundles = Vec::new();
        let mut attachments = Vec::new();

        // Iterate by index to preserve original entry order.
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).map(|e| e.name().to_owned()))
            .collect::<Result<Vec<_>, _>>()?;

        for name in names {
            let mut entry = zip.by_name(&name)?;
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buf)?;
            let ext = Path::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if matches!(ext, "reqif" | "xml") {
                let bundle = ReqIfParser::parse_bytes(&buf)?;
                bundles.push((name, bundle));
            } else {
                attachments.push((name, buf));
            }
        }
        Ok(Self {
            bundles,
            attachments,
        })
    }

    /// Serialize the bundle to a `.reqifz` file: bundles are written first
    /// (in `self.bundles` order), then attachments (in `self.attachments`
    /// order). Each entry uses its stored name verbatim.
    pub fn write(&self, path: impl AsRef<Path>, mode: FormatMode) -> Result<(), ReqIfError> {
        let file = File::create(path)?;
        let mut zw = ZipWriter::new(file);
        let opts: SimpleFileOptions = SimpleFileOptions::default();
        for (name, bundle) in &self.bundles {
            zw.start_file(name, opts)?;
            let xml = ReqIfUnparser::unparse(bundle, mode)?;
            zw.write_all(xml.as_bytes())?;
        }
        for (name, bytes) in &self.attachments {
            zw.start_file(name, opts)?;
            zw.write_all(bytes)?;
        }
        zw.finish()?;
        Ok(())
    }
}
