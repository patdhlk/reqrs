# reqrs

**reqrs** (pronounced "Wreckers", from Req + rs) is a Rust library and CLI for the
[ReqIF](https://www.omg.org/spec/ReqIF) requirements interchange format. It is a
port of the Python [`strict-doc-reqif`](https://github.com/strictdoc-project/reqif)
library and aims at byte-identical round-trip on real-world ReqIF files.

## Status

- 23/23 vendor `.reqif` fixtures round-trip byte-identically (Polarion, Doors,
  ReqIF Studio, Enterprise Architect, RMF, ci.eclipse.org)
- 3/3 `.reqifz` zip-bundle fixtures round-trip
- All Python CLI subcommands present: `passthrough`, `format`, `anonymize`,
  `dump`, `validate`, `version`
- Anonymizer output is byte-identical to Python at the default `--seed 0`

## Install

### CLI binary

```bash
cargo install --path crates/reqrs-cli
```

(Once published: `cargo install reqrs-cli`.)

### Library

```toml
[dependencies]
reqrs = "0.1"
```

## CLI usage

```bash
reqrs passthrough input.reqif output.reqif   # parse + unparse (round-trip test)
reqrs format      input.reqif output.reqif   # parse + canonical-indent re-emit
reqrs anonymize   input.reqif output.reqif   # replace user-visible strings
reqrs anonymize --seed 42 input.reqif output.reqif   # seeded for cross-run variation
reqrs dump        input.reqif output.html    # render bundle to HTML
reqrs validate    input.reqif                # internal semantic checks
reqrs validate --use-reqif-schema input.reqif   # plus XSD validation (requires xmllint)
reqrs version
```

## Library usage

```rust
use reqrs::{ReqIfParser, ReqIfUnparser, FormatMode};

fn main() -> anyhow::Result<()> {
    let bundle = ReqIfParser::parse_path("input.reqif")?;

    if let Some(header) = &bundle.header {
        println!("Document IDENTIFIER: {}", header.identifier);
        if let Some(title) = &header.title {
            println!("Title: {title}");
        }
    }

    if let Some(cc) = &bundle.core_content {
        if let Some(content) = &cc.req_if_content {
            if let Some(objects) = &content.spec_objects {
                println!("{} spec objects", objects.len());
            }
        }
    }

    // Write back out, byte-identical (FormatMode::Passthrough) or canonical
    let xml = ReqIfUnparser::unparse(&bundle, FormatMode::Passthrough)?;
    std::fs::write("output.reqif", xml)?;
    Ok(())
}
```

For `.reqifz` zip bundles:

```rust
use reqrs::{ReqIfzBundle, FormatMode};

let bundle = ReqIfzBundle::read("input.reqifz")?;
for (name, _) in &bundle.bundles {
    println!("inner .reqif: {name}");
}
for (name, bytes) in &bundle.attachments {
    println!("attachment: {name} ({} bytes)", bytes.len());
}
bundle.write("output.reqifz", FormatMode::Passthrough)?;
```

## How it compares to `strict-doc-reqif`

reqrs is a behavioral port — same XML output on the same input — with idiomatic
Rust shape. Key differences:

- **Enums per heterogeneous family.** Python uses dynamic types (`isinstance`
  dispatch); reqrs uses `enum DataType { String(...), Integer(...), ... }` and
  similar for `AttributeDefinition`, `AttributeValue`, `SpecType`. Exhaustive
  pattern matching gives compile-time coverage of every variant.
- **Newtype IDs.** `SpecObjectId`, `DataTypeId`, etc. prevent accidental
  cross-family ID mixing.
- **`Result`-based parsing.** Recoverable issues surface as `SchemaWarning`s on
  the bundle's `exceptions` field, mirroring Python's `bundle.exceptions`.

A small set of intentional deviations is documented inline in module docs:

- Anonymizer with non-zero `--seed` mixes seed bytes into SHA-256 input
  (Python doesn't have this knob; default `--seed 0` is byte-equivalent).
- XML datetime fields are stored as `Option<String>` for byte-fidelity;
  the `reqrs::helpers::datetime` module provides on-demand `chrono` parsing.

## Schema validation

`reqrs validate --use-reqif-schema` invokes `xmllint` against the embedded
OMG ReqIF XSD tree (and the imported XHTML modularization schemas). Install
`libxml2-utils` (Debian/Ubuntu) or `libxml2` via Homebrew (macOS) to enable
this path. The internal semantic checks (duplicate identifiers, dangling
references, missing XML declaration, non-UTF-8 encoding) run regardless.

## License

Apache-2.0. See [LICENSE](LICENSE).

## Acknowledgments

reqrs is a direct port of [`strict-doc-reqif`](https://github.com/strictdoc-project/reqif)
by Stanislav Pankevich and the StrictDoc team. The integration test corpus
in `tests/corpus/` is copied verbatim from that project.
