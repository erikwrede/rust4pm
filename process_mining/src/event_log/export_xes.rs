use std::{
    borrow::Borrow,
    fs::File,
    io::{BufWriter, Write},
};

use flate2::{write::GzEncoder, Compression};
use quick_xml::{events::BytesDecl, Writer};

use crate::EventLog;

use super::{
    event_log_struct::{EventLogClassifier, EventLogExtension},
    stream_xes::{XESOuterLogData, XESParsingTraceStream},
    Attribute, Attributes, Trace,
};
const OK: Result<(), quick_xml::Error> = Ok::<(), quick_xml::Error>(());

pub fn export_xes<'a, W, T: Borrow<Trace>, I>(
    writer: &mut Writer<W>,
    log_extensions: &'a Option<&'a Vec<EventLogExtension>>,
    log_global_trace_attrs: &'a Option<&'a Attributes>,
    log_global_event_attrs: &'a Option<&'a Attributes>,
    log_classifiers: &'a Option<&'a Vec<EventLogClassifier>>,
    log_attributes: &'a Attributes,
    traces: I,
) -> Result<(), quick_xml::Error>
where
    I: Iterator<Item = T>,
    W: Write,
{
    writer
        .write_event(quick_xml::events::Event::Decl(BytesDecl::new(
            "1.0",
            Some("utf8"),
            None,
        )))
        .unwrap();
    writer
        .create_element("log")
        .with_attributes(vec![
            ("xes.version", "2.0"),
            // nested-attributes are not always present, but they might be so let's just say we use them
            ("xes.features", "nested-attributes"),
            ("xmlns", "http://www.xes-standard.org/"),
        ])
        .write_inner_content(|w| {
            // Extensions
            if let Some(extensions) = log_extensions {
                for ext in extensions.iter() {
                    w.create_element("extension")
                        .with_attributes(vec![
                            ("name", ext.name.as_str()),
                            ("prefix", ext.prefix.as_str()),
                            ("uri", ext.uri.as_str()),
                        ])
                        .write_empty()?;
                }
            }
            // Global trace attributes
            if let Some(global_trace_attrs) = log_global_trace_attrs {
                w.create_element("global")
                    .with_attribute(("scope", "trace"))
                    .write_inner_content(|w| {
                        for a in global_trace_attrs.iter() {
                            write_xes_attribute(w, a)?;
                        }
                        OK
                    })?;
            }
            // Global event attributes
            if let Some(global_event_attrs) = log_global_event_attrs {
                w.create_element("global")
                    .with_attribute(("scope", "event"))
                    .write_inner_content(|w| {
                        for a in global_event_attrs.iter() {
                            write_xes_attribute(w, a)?;
                        }
                        OK
                    })?;
            }
            // Classifiers
            if let Some(classifiers) = log_classifiers {
                for cl in classifiers.iter() {
                    w.create_element("classifier")
                        .with_attributes(vec![
                            ("name", cl.name.as_str()),
                            ("keys", cl.keys.join(" ").as_str()),
                        ])
                        .write_empty()?;
                }
            }
            // Log attributes
            for a in log_attributes {
                write_xes_attribute(w, a)?;
            }
            for t in traces {
                w.create_element("trace").write_inner_content(|w| {
                    for a in &t.borrow().attributes {
                        write_xes_attribute(w, a)?;
                    }
                    for e in &t.borrow().events {
                        w.create_element("event").write_inner_content(|w| {
                            for a in &e.attributes {
                                write_xes_attribute(w, a)?;
                            }
                            OK
                        })?;
                    }
                    OK
                })?;
            }
            OK
        })?;

    OK
}

fn write_xes_attribute<T>(w: &mut Writer<T>, a: &Attribute) -> Result<(), quick_xml::Error>
where
    T: Write,
{
    let (tag_name, value_opt): (&str, Option<String>) = match &a.value {
        super::AttributeValue::String(s) => ("string", Some(s.clone())),
        super::AttributeValue::Date(d) => ("date", Some(d.to_rfc3339())),
        super::AttributeValue::Int(i) => ("int", Some(i.to_string())),
        super::AttributeValue::Float(f) => ("float", Some(f.to_string())),
        super::AttributeValue::Boolean(b) => ("boolean", Some(b.to_string())),
        super::AttributeValue::ID(id) => ("id", Some(id.to_string())),
        super::AttributeValue::List(_) => ("list", None),
        super::AttributeValue::Container(_) => ("container", None),
        super::AttributeValue::None() => todo!("None AttributeValue encountered"),
    };
    let e = match value_opt {
        Some(value) => w
            .create_element(tag_name)
            .with_attributes(vec![("key", a.key.as_str()), ("value", &value)]),
        None => w
            .create_element(tag_name)
            .with_attribute(("key", a.key.as_str())),
    };
    if let Some(own_nested_attrs) = &a.own_attributes {
        e.write_inner_content(|inner_w| {
            for own_attr in own_nested_attrs {
                write_xes_attribute(inner_w, own_attr)?;
            }
            OK
        })?;
    } else {
        e.write_empty()?;
    }

    OK
}

pub fn export_xes_event_log<T>(
    writer: &mut Writer<T>,
    log: &EventLog,
) -> Result<(), quick_xml::Error>
where
    T: Write,
{
    export_xes(
        writer,
        &log.extensions.as_ref(),
        &log.global_trace_attrs.as_ref(),
        &log.global_event_attrs.as_ref(),
        &log.classifiers.as_ref(),
        &log.attributes,
        log.traces.iter(),
    )
}

pub fn export_xes_event_log_to_file(
    log: &EventLog,
    file: File,
    compress_gz: bool,
) -> Result<(), quick_xml::Error> {
    if compress_gz {
        let encoder = GzEncoder::new(BufWriter::new(file), Compression::fast());
        return export_xes_event_log(&mut Writer::new(BufWriter::new(encoder)), &log);
    }
    export_xes_event_log(&mut Writer::new(BufWriter::new(file)), &log)
}

pub fn export_xes_event_log_to_file_path(
    log: &EventLog,
    path: &str,
) -> Result<(), quick_xml::Error> {
    let file = File::create(path)?;
    export_xes_event_log_to_file(log, file, path.ends_with(".gz"))
}

pub fn export_xes_trace_stream<T>(
    writer: &mut Writer<T>,
    mut trace_stream: XESParsingTraceStream,
    log_data: XESOuterLogData,
) -> Result<(), quick_xml::Error>
where
    T: Write,
{
    export_xes(
        writer,
        &Some(log_data.extensions.as_ref()),
        &Some(log_data.global_trace_attrs.as_ref()),
        &Some(log_data.global_event_attrs.as_ref()),
        &Some(log_data.classifiers.as_ref()),
        &log_data.log_attributes,
        trace_stream.into_iter(),
    )
}

pub fn export_xes_trace_stream_to_file(
    trace_stream: XESParsingTraceStream,
    log_data: XESOuterLogData,
    file: File,
    compress_gz: bool,
) -> Result<(), quick_xml::Error> {
    if compress_gz {
        let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        return export_xes_trace_stream(
            &mut Writer::new(BufWriter::new(encoder)),
            trace_stream,
            log_data,
        );
    }
    export_xes_trace_stream(
        &mut Writer::new(BufWriter::new(file)),
        trace_stream,
        log_data,
    )
}

#[cfg(test)]
mod export_xes_tests {
    use std::{collections::HashSet, fs::File, time::Instant};

    
    use quick_xml::Writer;

    use crate::{
        event_log::{
            event_log_struct::EventLogExtension,
            export_xes::export_xes_event_log,
        }, stream_xes_slice_gz,
        XESImportOptions,
    };

    use super::export_xes_trace_stream_to_file;

    #[test]
    fn test_xes_export() {
        let x = include_bytes!("./tests/test_data/Sepsis Cases - Event Log.xes.gz");
        let log = crate::import_xes_slice(x, true, crate::XESImportOptions::default()).unwrap();
        let exported_xes_data: Vec<u8> = Vec::new();
        let mut writer = Writer::new(exported_xes_data);
        export_xes_event_log(&mut writer, &log).unwrap();
        let data = writer.into_inner();
        let log2 =
            crate::import_xes_slice(&data, false, crate::XESImportOptions::default()).unwrap();
        assert_eq!(log.traces.len(), log2.traces.len());
        assert_eq!(log.attributes.len(), log2.attributes.len());
        assert_eq!(
            log.classifiers
                .as_ref()
                .map(|c| c.len())
                .unwrap_or_default(),
            log2.classifiers
                .as_ref()
                .map(|c| c.len())
                .unwrap_or_default(),
        );
        assert_eq!(
            log.extensions
                .as_ref()
                .unwrap()
                .iter()
                .collect::<HashSet<&EventLogExtension>>(),
            log2.extensions
                .as_ref()
                .unwrap()
                .iter()
                .collect::<HashSet<&EventLogExtension>>()
        );

        // The below assumes that also all orders of events, traces, log attributes, extensions etc. must be the same
        // In reality, we would also accept a weaker equality relation (e.g., ignoring the order of attributes)
        assert!(log2 == log);
    }

    #[test]
    fn test_stream_from_gz_to_plain() {
        let now = Instant::now();
        let data = include_bytes!("./tests/test_data/Road_Traffic_Fine_Management_Process.xes.gz");

        let (stream, log_data) = stream_xes_slice_gz(data, XESImportOptions::default()).unwrap();
        let file = File::create("./streaming-export.xes").unwrap();
        export_xes_trace_stream_to_file(stream, log_data, file, false).unwrap();
        println!("Streamed from .xes.gz to .xes in {:?}", now.elapsed());
    }
}
