// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Phase A incremental-repair benchmarks.
//!
//! Compares `Document::set` (which routes through the localised
//! `replace_span` repair) against a synthetic baseline that
//! simulates the pre-Phase-A behaviour by full-re-parsing the
//! post-edit source string.
//!
//! What the numbers mean:
//!   * `phase_a_set` — current behaviour. Validation pass +
//!     localised green-tree repair.
//!   * `baseline_full_reparse` — the ceiling the old behaviour
//!     would have hit. Pure `parse_document(new_source)` at
//!     each iteration. (The Document is reconstructed from the
//!     new source — same bytes-out as Phase A.)
//!
//! Run: `cargo bench --bench incremental_repair`

#![allow(missing_docs, unused_results)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use noyalib::cst::parse_document;

/// Build a synthetic block-mapping document with `n_entries`
/// keys. Each value is a small plain scalar; the document is
/// roughly `~32 * n_entries` bytes.
fn synth_doc(n_entries: usize) -> String {
    let mut out = String::with_capacity(n_entries * 32);
    for i in 0..n_entries {
        out.push_str(&format!("key_{i:05}: value_{i:05}\n"));
    }
    out
}

fn bench_value_bump_at(c: &mut Criterion, target: &str, n_entries_list: &[usize]) {
    let mut group = c.benchmark_group(format!("value_bump_at_{target}"));
    for &n in n_entries_list {
        let src = synth_doc(n);
        let bytes = src.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        // Pick a single representative key per group.
        let key_idx = match target {
            "first" => 0,
            "middle" => n / 2,
            "last" => n - 1,
            _ => 0,
        };
        let key = format!("key_{key_idx:05}");
        let new_val = "bumped_value";

        // Phase A: cold setup, hot edit.
        group.bench_with_input(
            BenchmarkId::new("phase_a_set", n),
            &(src.clone(), key.clone(), new_val),
            |b, (src, key, new_val)| {
                b.iter_with_setup(
                    || parse_document(src).unwrap(),
                    |mut doc| {
                        doc.set(black_box(key), black_box(new_val)).unwrap();
                        black_box(doc)
                    },
                );
            },
        );

        // Synthetic baseline: full re-parse of the post-edit
        // source. This is what `replace_span` did before Phase A
        // (well, plus an extra parse for the green tree on top).
        // Even with this conservative baseline (one parse, not
        // two), the comparison shows the parse-vs-walk gap.
        group.bench_with_input(
            BenchmarkId::new("baseline_full_reparse", n),
            &(src.clone(), key.clone(), new_val),
            |b, (src, key, new_val)| {
                b.iter_with_setup(
                    || {
                        // Pre-compute the post-edit source so the
                        // measured iteration is just the full
                        // parse, mirroring the dominant cost.
                        let doc = parse_document(src).unwrap();
                        let (s, e) = doc.span_at(key).unwrap();
                        let mut new_src = String::with_capacity(src.len() + 16);
                        new_src.push_str(&src[..s]);
                        new_src.push_str(new_val);
                        new_src.push_str(&src[e..]);
                        new_src
                    },
                    |new_src| black_box(parse_document(black_box(&new_src)).unwrap()),
                );
            },
        );
    }
    group.finish();
}

fn bench_phase_a(c: &mut Criterion) {
    let sizes = [50usize, 500, 5_000];
    bench_value_bump_at(c, "first", &sizes);
    bench_value_bump_at(c, "middle", &sizes);
    bench_value_bump_at(c, "last", &sizes);
}

criterion_group!(name = benches; config = Criterion::default(); targets = bench_phase_a);
criterion_main!(benches);
