use std::fs;
use std::path::PathBuf;

use crate::error::ReqIfError;
use crate::parse::ReqIfParser;
use crate::unparse::{FormatMode, ReqIfUnparser};

#[derive(Debug, Clone)]
pub struct PassthroughOpts {
    pub input: PathBuf,
    pub output: PathBuf,
}

pub fn passthrough(opts: PassthroughOpts) -> Result<(), ReqIfError> {
    let bundle = ReqIfParser::parse_path(&opts.input)?;
    let out = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough)?;
    fs::write(&opts.output, out)?;
    Ok(())
}
