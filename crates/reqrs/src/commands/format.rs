use std::fs;
use std::path::PathBuf;

use crate::error::ReqIfError;
use crate::parse::ReqIfParser;
use crate::unparse::{FormatMode, ReqIfUnparser};

#[derive(Debug, Clone)]
pub struct FormatOpts {
    pub input: PathBuf,
    pub output: PathBuf,
}

pub fn format(opts: FormatOpts) -> Result<(), ReqIfError> {
    let bundle = ReqIfParser::parse_path(&opts.input)?;
    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Canonical)?;
    fs::write(&opts.output, out)?;
    Ok(())
}
