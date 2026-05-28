use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use crate::error::ReqIfError;
use crate::model::{ReqIfBundle, SpecType};
use crate::parse::ReqIfParser;

#[derive(Debug, Clone)]
pub struct DumpOpts {
    pub input: PathBuf,
    pub output: PathBuf,
}

pub fn dump(opts: DumpOpts) -> Result<(), ReqIfError> {
    let bundle = ReqIfParser::parse_path(&opts.input)?;
    let html = render_html(&bundle);
    fs::write(&opts.output, html)?;
    Ok(())
}

fn render_html(bundle: &ReqIfBundle) -> String {
    let mut out = String::new();
    out.push_str(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>ReqIF dump</title></head><body>\n",
    );

    if let Some(h) = &bundle.header {
        let _ = writeln!(out, "<h1>{}</h1>", html_escape(&h.identifier));
        if let Some(t) = &h.title {
            let _ = writeln!(out, "<p>Title: {}</p>", html_escape(t));
        }
    }

    if let Some(cc) = &bundle.core_content
        && let Some(content) = &cc.req_if_content
    {
        if let Some(spec_types) = &content.spec_types {
            out.push_str("<h2>Spec types</h2><ul>\n");
            for st in spec_types {
                let kind = match st {
                    SpecType::SpecObject(_) => "SpecObject",
                    SpecType::Specification(_) => "Specification",
                    SpecType::SpecRelation(_) => "SpecRelation",
                    SpecType::RelationGroup(_) => "RelationGroup",
                };
                let _ = writeln!(
                    out,
                    "  <li>{} ({})</li>",
                    html_escape(st.identifier().as_str()),
                    kind
                );
            }
            out.push_str("</ul>\n");
        }

        if let Some(objs) = &content.spec_objects {
            let _ = writeln!(out, "<h2>Spec objects ({})</h2><ul>", objs.len());
            for o in objs {
                let _ = writeln!(out, "  <li>{}</li>", html_escape(o.identifier.as_str()));
            }
            out.push_str("</ul>\n");
        }

        if let Some(specs) = &content.specifications {
            let _ = writeln!(out, "<h2>Specifications ({})</h2><ul>", specs.len());
            for s in specs {
                let _ = writeln!(out, "  <li>{}</li>", html_escape(s.identifier.as_str()));
            }
            out.push_str("</ul>\n");
        }
    }

    out.push_str("</body></html>\n");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
