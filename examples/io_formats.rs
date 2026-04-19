// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! I/O format examples: from_slice, from_reader, to_writer, to_fmt_writer.
//!
//! Run: `cargo run --example io_formats`

use std::io::Cursor;

fn done(msg: &str) {
    println!("  \x1b[32m+\x1b[0m {msg}");
}

fn main() {
    println!("\n  \x1b[1mnoyalib i/o formats\x1b[0m\n");

    let yaml_bytes = b"name: noyalib\nversion: 1\n";
    let _: noyalib::Value = noyalib::from_slice(yaml_bytes).unwrap();
    done("from_slice");

    let config = noyalib::ParserConfig::strict();
    let _: noyalib::Value = noyalib::from_slice_with_config(yaml_bytes, &config).unwrap();
    done("from_slice_with_config (strict)");

    let reader = Cursor::new("host: localhost\nport: 8080\n");
    let _: noyalib::Value = noyalib::from_reader(reader).unwrap();
    done("from_reader");

    let reader = Cursor::new("key: value\n");
    let _: noyalib::Value = noyalib::from_reader_with_config(reader, &config).unwrap();
    done("from_reader_with_config");

    let data = noyalib::Value::String("hello".to_string());
    let mut buf = Vec::new();
    noyalib::to_writer(&mut buf, &data).unwrap();
    done(&format!("to_writer ({} bytes)", buf.len()));

    let ser_config = noyalib::SerializerConfig::new().indent(4);
    let mut buf = Vec::new();
    noyalib::to_writer_with_config(&mut buf, &data, &ser_config).unwrap();
    done("to_writer_with_config (indent=4)");

    let mut output = String::new();
    noyalib::to_fmt_writer(&mut output, &data).unwrap();
    done("to_fmt_writer");

    let mut output = String::new();
    let ser_config = noyalib::SerializerConfig::new().document_start(true);
    noyalib::to_fmt_writer_with_config(&mut output, &data, &ser_config).unwrap();
    done("to_fmt_writer_with_config (document_start)");

    println!("\n  \x1b[90mAll I/O formats verified.\x1b[0m\n");
}
