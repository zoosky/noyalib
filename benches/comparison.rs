//! Head-to-head benchmarks: noyalib vs serde_yaml_ng vs yaml-rust2.
//!
//! Run: `cargo bench --bench comparison`

#![allow(missing_docs, unused_results)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};

// ── Test Data ────────────────────────────────────────────────────────

const SIMPLE: &str = "name: myapp\nversion: 1\nenabled: true\n";

const NESTED: &str = "\
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    cert: /etc/ssl/cert.pem
    key: /etc/ssl/key.pem
database:
  host: db.example.com
  port: 5432
  name: production
  pool:
    min: 5
    max: 25
    timeout: 30
logging:
  level: info
  format: json
  outputs:
    - type: stdout
    - type: file
      path: /var/log/app.log
";

const LARGE_LIST: &str = include_str!("fixtures/large_list.yaml");

#[derive(Debug, Serialize, Deserialize)]
struct Simple {
    name: String,
    version: u32,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Nested {
    server: Server,
    database: Database,
}

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    host: String,
    port: u16,
    ssl: Ssl,
}

#[derive(Debug, Serialize, Deserialize)]
struct Ssl {
    enabled: bool,
    cert: String,
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Database {
    host: String,
    port: u16,
    name: String,
}

// ── Deserialization Benchmarks ───────────────────────────────────────

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    for (name, yaml) in [
        ("simple", SIMPLE),
        ("nested", NESTED),
        ("large_list", LARGE_LIST),
    ] {
        group.bench_with_input(BenchmarkId::new("noyalib", name), yaml, |b, input| {
            b.iter(|| {
                let _: noyalib::Value = noyalib::from_str(black_box(input)).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("serde_yaml_ng", name), yaml, |b, input| {
            b.iter(|| {
                let _: serde_yaml_ng::Value = serde_yaml_ng::from_str(black_box(input)).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("yaml-rust2", name), yaml, |b, input| {
            b.iter(|| {
                let _ = yaml_rust2::YamlLoader::load_from_str(black_box(input)).unwrap();
            });
        });
    }

    group.finish();
}

// ── Typed Deserialization Benchmarks ─────────────────────────────────

fn bench_typed_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("typed_deserialize");

    group.bench_function("noyalib/simple", |b| {
        b.iter(|| {
            let _: Simple = noyalib::from_str(black_box(SIMPLE)).unwrap();
        });
    });

    group.bench_function("serde_yaml_ng/simple", |b| {
        b.iter(|| {
            let _: Simple = serde_yaml_ng::from_str(black_box(SIMPLE)).unwrap();
        });
    });

    group.bench_function("noyalib/nested", |b| {
        b.iter(|| {
            let _: Nested = noyalib::from_str(black_box(NESTED)).unwrap();
        });
    });

    group.bench_function("serde_yaml_ng/nested", |b| {
        b.iter(|| {
            let _: Nested = serde_yaml_ng::from_str(black_box(NESTED)).unwrap();
        });
    });

    group.finish();
}

// ── Serialization Benchmarks ─────────────────────────────────────────

fn bench_serialize(c: &mut Criterion) {
    let noya_simple: noyalib::Value = noyalib::from_str(SIMPLE).unwrap();
    let noya_nested: noyalib::Value = noyalib::from_str(NESTED).unwrap();
    let serde_simple: serde_yaml_ng::Value = serde_yaml_ng::from_str(SIMPLE).unwrap();
    let serde_nested: serde_yaml_ng::Value = serde_yaml_ng::from_str(NESTED).unwrap();

    let mut group = c.benchmark_group("serialize");

    group.bench_function("noyalib/simple", |b| {
        b.iter(|| {
            let _ = noyalib::to_string(black_box(&noya_simple)).unwrap();
        });
    });

    group.bench_function("serde_yaml_ng/simple", |b| {
        b.iter(|| {
            let _ = serde_yaml_ng::to_string(black_box(&serde_simple)).unwrap();
        });
    });

    group.bench_function("noyalib/nested", |b| {
        b.iter(|| {
            let _ = noyalib::to_string(black_box(&noya_nested)).unwrap();
        });
    });

    group.bench_function("serde_yaml_ng/nested", |b| {
        b.iter(|| {
            let _ = serde_yaml_ng::to_string(black_box(&serde_nested)).unwrap();
        });
    });

    group.finish();
}

// ── Roundtrip Benchmarks ─────────────────────────────────────────────

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    group.bench_function("noyalib/nested", |b| {
        b.iter(|| {
            let v: noyalib::Value = noyalib::from_str(black_box(NESTED)).unwrap();
            let _ = noyalib::to_string(&v).unwrap();
        });
    });

    group.bench_function("serde_yaml_ng/nested", |b| {
        b.iter(|| {
            let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(black_box(NESTED)).unwrap();
            let _ = serde_yaml_ng::to_string(&v).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_deserialize,
    bench_typed_deserialize,
    bench_serialize,
    bench_roundtrip,
);
criterion_main!(benches);
