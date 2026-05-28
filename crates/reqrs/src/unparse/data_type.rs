use crate::model::data_type::*;
use crate::unparse::writer::{escape_attr, write_close, write_open, write_self_closing};

const INDENT: &str = "        ";

pub fn unparse_data_type(dt: &DataType) -> String {
    match dt {
        DataType::String(d) => unparse_string(d),
        DataType::Boolean(d) => unparse_simple(
            d.identifier.as_str(),
            &d.common,
            "DATATYPE-DEFINITION-BOOLEAN",
            &[],
        ),
        DataType::Integer(d) => unparse_simple(
            d.identifier.as_str(),
            &d.common,
            "DATATYPE-DEFINITION-INTEGER",
            &[
                ("MAX", d.max_value.as_deref()),
                ("MIN", d.min_value.as_deref()),
            ],
        ),
        DataType::Real(d) => {
            // Python always emits Real self-closed.
            let mut common = d.common.clone();
            common.was_self_closing = true;
            unparse_simple(
                d.identifier.as_str(),
                &common,
                "DATATYPE-DEFINITION-REAL",
                &[
                    ("ACCURACY", d.accuracy.as_deref()),
                    ("MAX", d.max_value.as_deref()),
                    ("MIN", d.min_value.as_deref()),
                ],
            )
        }
        DataType::Date(d) => unparse_simple(
            d.identifier.as_str(),
            &d.common,
            "DATATYPE-DEFINITION-DATE",
            &[],
        ),
        DataType::Xhtml(d) => unparse_simple(
            d.identifier.as_str(),
            &d.common,
            "DATATYPE-DEFINITION-XHTML",
            &[],
        ),
        DataType::Enumeration(d) => unparse_enumeration(d),
    }
}

fn unparse_string(d: &DataTypeString) -> String {
    unparse_simple(
        d.identifier.as_str(),
        &d.common,
        "DATATYPE-DEFINITION-STRING",
        &[("MAX-LENGTH", d.max_length.as_deref())],
    )
}

fn collect_attrs<'a>(
    identifier: &'a str,
    common: &'a DataTypeCommon,
    extras: &'a [(&'a str, Option<&'a str>)],
) -> Vec<(&'a str, String)> {
    let mut attrs: Vec<(&str, String)> = Vec::with_capacity(4 + extras.len());
    if let Some(d) = &common.description {
        attrs.push(("DESC", d.clone()));
    }
    attrs.push(("IDENTIFIER", identifier.to_owned()));
    if let Some(d) = &common.last_change {
        attrs.push(("LAST-CHANGE", d.clone()));
    }
    if let Some(d) = &common.long_name {
        attrs.push(("LONG-NAME", d.clone()));
    }
    for (k, v) in extras {
        if let Some(v) = v {
            attrs.push((k, (*v).to_owned()));
        }
    }
    attrs
}

fn unparse_simple(
    identifier: &str,
    common: &DataTypeCommon,
    tag: &str,
    extras: &[(&str, Option<&str>)],
) -> String {
    let mut attrs = collect_attrs(identifier, common, extras);

    let mut out = String::new();
    if common.was_self_closing {
        write_self_closing(&mut out, INDENT, tag, &mut attrs)
            .expect("writing to String never fails");
    } else {
        write_open(&mut out, INDENT, tag, &mut attrs).expect("writing to String never fails");
        write_close(&mut out, INDENT, tag);
    }
    out
}

fn unparse_enumeration(d: &DataTypeEnumeration) -> String {
    let mut attrs = collect_attrs(d.identifier.as_str(), &d.common, &[]);

    let mut out = String::new();
    if d.common.was_self_closing && d.specified_values.is_none() {
        write_self_closing(
            &mut out,
            INDENT,
            "DATATYPE-DEFINITION-ENUMERATION",
            &mut attrs,
        )
        .expect("writing to String never fails");
        return out;
    }
    write_open(
        &mut out,
        INDENT,
        "DATATYPE-DEFINITION-ENUMERATION",
        &mut attrs,
    )
    .expect("writing to String never fails");
    if let Some(values) = &d.specified_values {
        out.push_str("          <SPECIFIED-VALUES>\n");
        for v in values {
            out.push_str("            <ENUM-VALUE");
            let mut ev_attrs: Vec<(&str, String)> = Vec::new();
            if let Some(s) = &v.description {
                ev_attrs.push(("DESC", s.clone()));
            }
            ev_attrs.push(("IDENTIFIER", v.identifier.as_str().to_owned()));
            if let Some(s) = &v.last_change {
                ev_attrs.push(("LAST-CHANGE", s.clone()));
            }
            if let Some(s) = &v.long_name {
                ev_attrs.push(("LONG-NAME", s.clone()));
            }
            ev_attrs.sort_by(|a, b| a.0.cmp(b.0));
            for (k, val) in &ev_attrs {
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                escape_attr(&mut out, val);
                out.push('"');
            }
            out.push_str(">\n");
            out.push_str("              <PROPERTIES>\n");
            out.push_str("                <EMBEDDED-VALUE KEY=\"");
            escape_attr(&mut out, &v.key);
            out.push('"');
            if let Some(oc) = &v.other_content {
                out.push_str(" OTHER-CONTENT=\"");
                escape_attr(&mut out, oc);
                out.push('"');
            }
            out.push_str("/>\n");
            out.push_str("              </PROPERTIES>\n");
            out.push_str("            </ENUM-VALUE>\n");
        }
        out.push_str("          </SPECIFIED-VALUES>\n");
    }
    write_close(&mut out, INDENT, "DATATYPE-DEFINITION-ENUMERATION");
    out
}
