use crate::error::ReqIfError;
use crate::ids::{DataTypeId, EnumValueId};
use crate::model::data_type::*;
use crate::parse::reader::{ReqIfReader, optional_attr, required_attr};
use quick_xml::events::{BytesStart, Event};

pub fn parse_data_type(xml: &str) -> Result<DataType, ReqIfError> {
    let mut r = ReqIfReader::new(xml.as_bytes());
    loop {
        match r.read_event()? {
            Event::Start(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_data_type_inner(&mut r, &owned, &tag, false);
            }
            Event::Empty(s) => {
                let tag = s.name().as_ref().to_vec();
                let owned = s.into_owned();
                return parse_data_type_inner(&mut r, &owned, &tag, true);
            }
            Event::Eof => {
                return Err(ReqIfError::MissingChild {
                    child: "DATATYPE-DEFINITION-*".into(),
                    parent: "<root>".into(),
                });
            }
            _ => continue,
        }
    }
}

pub(crate) fn parse_data_type_inner(
    r: &mut ReqIfReader<'_>,
    start: &BytesStart<'_>,
    tag: &[u8],
    was_self_closing: bool,
) -> Result<DataType, ReqIfError> {
    let identifier = DataTypeId(required_attr(start, "IDENTIFIER")?);
    let common = DataTypeCommon {
        description: optional_attr(start, "DESC"),
        last_change: optional_attr(start, "LAST-CHANGE"),
        long_name: optional_attr(start, "LONG-NAME"),
        was_self_closing,
    };

    let dt = match tag {
        b"DATATYPE-DEFINITION-STRING" => DataType::String(DataTypeString {
            identifier,
            common,
            max_length: optional_attr(start, "MAX-LENGTH"),
        }),
        b"DATATYPE-DEFINITION-BOOLEAN" => DataType::Boolean(DataTypeBoolean { identifier, common }),
        b"DATATYPE-DEFINITION-INTEGER" => DataType::Integer(DataTypeInteger {
            identifier,
            common,
            max_value: optional_attr(start, "MAX"),
            min_value: optional_attr(start, "MIN"),
        }),
        b"DATATYPE-DEFINITION-REAL" => DataType::Real(DataTypeReal {
            identifier,
            common,
            accuracy: optional_attr(start, "ACCURACY"),
            max_value: optional_attr(start, "MAX"),
            min_value: optional_attr(start, "MIN"),
        }),
        b"DATATYPE-DEFINITION-DATE" => DataType::Date(DataTypeDate { identifier, common }),
        b"DATATYPE-DEFINITION-XHTML" => DataType::Xhtml(DataTypeXhtml { identifier, common }),
        b"DATATYPE-DEFINITION-ENUMERATION" => {
            let specified_values = if was_self_closing {
                None
            } else {
                parse_enum_specified_values(r)?
            };
            DataType::Enumeration(DataTypeEnumeration {
                identifier,
                common,
                specified_values,
            })
        }
        _ => {
            return Err(ReqIfError::UnexpectedTag {
                tag: String::from_utf8_lossy(tag).into_owned(),
                parent: "DATATYPES".into(),
            });
        }
    };

    if !was_self_closing && !matches!(dt, DataType::Enumeration(_)) {
        // Consume until the matching close tag (non-enum variants have no body).
        let end_name = tag.to_vec();
        loop {
            match r.read_event()? {
                Event::End(e) if e.name().as_ref() == end_name.as_slice() => break,
                Event::Eof => {
                    return Err(ReqIfError::Xml {
                        pos: r.buffer_position(),
                        msg: format!("EOF inside <{}>", String::from_utf8_lossy(&end_name)),
                    });
                }
                _ => continue,
            }
        }
    }
    Ok(dt)
}

fn parse_enum_specified_values(
    r: &mut ReqIfReader<'_>,
) -> Result<Option<Vec<EnumValue>>, ReqIfError> {
    let mut values: Option<Vec<EnumValue>> = None;
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"SPECIFIED-VALUES" => {
                values = Some(parse_enum_values_list(r)?);
            }
            Event::Empty(s) if s.name().as_ref() == b"SPECIFIED-VALUES" => {
                values = Some(Vec::new());
            }
            Event::End(e) if e.name().as_ref() == b"DATATYPE-DEFINITION-ENUMERATION" => {
                return Ok(values);
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <DATATYPE-DEFINITION-ENUMERATION>".into(),
                });
            }
            _ => continue,
        }
    }
}

fn parse_enum_values_list(r: &mut ReqIfReader<'_>) -> Result<Vec<EnumValue>, ReqIfError> {
    let mut out = Vec::new();
    loop {
        match r.read_event()? {
            Event::Start(s) if s.name().as_ref() == b"ENUM-VALUE" => {
                let identifier = EnumValueId(required_attr(&s, "IDENTIFIER")?);
                let description = optional_attr(&s, "DESC");
                let last_change = optional_attr(&s, "LAST-CHANGE");
                let long_name = optional_attr(&s, "LONG-NAME");
                let (key, other_content) = parse_enum_value_properties(r)?;
                out.push(EnumValue {
                    identifier,
                    description,
                    last_change,
                    long_name,
                    key,
                    other_content,
                });
            }
            Event::End(e) if e.name().as_ref() == b"SPECIFIED-VALUES" => return Ok(out),
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <SPECIFIED-VALUES>".into(),
                });
            }
            _ => continue,
        }
    }
}

fn parse_enum_value_properties(
    r: &mut ReqIfReader<'_>,
) -> Result<(String, Option<String>), ReqIfError> {
    let mut key = None;
    let mut other_content = None;
    loop {
        match r.read_event()? {
            Event::Empty(s) if s.name().as_ref() == b"EMBEDDED-VALUE" => {
                key = Some(required_attr(&s, "KEY")?);
                other_content = optional_attr(&s, "OTHER-CONTENT");
            }
            Event::Start(s) if s.name().as_ref() == b"EMBEDDED-VALUE" => {
                key = Some(required_attr(&s, "KEY")?);
                other_content = optional_attr(&s, "OTHER-CONTENT");
            }
            Event::End(e) if e.name().as_ref() == b"ENUM-VALUE" => {
                let key = key.ok_or(ReqIfError::MissingChild {
                    child: "EMBEDDED-VALUE".into(),
                    parent: "PROPERTIES".into(),
                })?;
                return Ok((key, other_content));
            }
            Event::Eof => {
                return Err(ReqIfError::Xml {
                    pos: r.buffer_position(),
                    msg: "EOF inside <ENUM-VALUE>".into(),
                });
            }
            _ => continue,
        }
    }
}
