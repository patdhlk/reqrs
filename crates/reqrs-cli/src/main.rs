use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use reqrs::commands::{anonymize, dump, format, passthrough, validate, version};

#[derive(Parser)]
#[command(
    name = "reqrs",
    version,
    about = "ReqIF parser/unparser (Rust port of strict-doc-reqif)"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Parse a ReqIF file, then write it back out (used for testing).
    Passthrough { input: PathBuf, output: PathBuf },
    /// Parse a ReqIF file and pretty-print it.
    Format { input: PathBuf, output: PathBuf },
    /// Parse and anonymize a ReqIF file.
    Anonymize {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = 0)]
        seed: u64,
    },
    /// Parse a ReqIF file and dump its contents to an HTML page.
    Dump { input: PathBuf, output: PathBuf },
    /// Parse a ReqIF file and validate it.
    Validate {
        input: PathBuf,
        #[arg(long)]
        use_reqif_schema: bool,
    },
    /// Print the reqrs version.
    Version,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Passthrough { input, output } => {
            passthrough::passthrough(passthrough::PassthroughOpts { input, output })?;
        }
        Cmd::Format { input, output } => {
            format::format(format::FormatOpts { input, output })?;
        }
        Cmd::Anonymize {
            input,
            output,
            seed,
        } => {
            anonymize::anonymize(anonymize::AnonymizeOpts {
                input,
                output,
                seed,
            })?;
        }
        Cmd::Dump { input, output } => {
            dump::dump(dump::DumpOpts { input, output })?;
        }
        Cmd::Validate {
            input,
            use_reqif_schema,
        } => {
            let report = validate::validate(validate::ValidateOpts {
                input,
                use_reqif_schema,
            })?;
            for e in &report.xml_errors {
                eprintln!("error: {e}");
            }
            for e in &report.schema_errors {
                eprintln!("warning: schema: {e}");
            }
            for e in &report.semantic_warnings {
                eprintln!("warning: semantic: {e}");
            }
            if report.has_any_errors() {
                return Ok(ExitCode::from(1));
            }
        }
        Cmd::Version => println!("{}", version::version()),
    }
    Ok(ExitCode::SUCCESS)
}
