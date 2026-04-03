// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! Ring buffer benchmarks

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rusty_lights::audio::RingBuffer;

fn bench_pop_slice(c: &mut Criterion) {
    let mut group = c.benchmark_group("RingBuffer/pop_slice");

    for size in [256, 512, 1024] {
        let rb: RingBuffer<f32> = RingBuffer::new(8192);
        let input: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let mut output = vec![0.0f32; size];

        group.bench_with_input(BenchmarkId::new("samples", size), &size, |b, _| {
            b.iter(|| {
                rb.push_slice(&input);
                black_box(rb.pop_slice(&mut output));
            });
        });
    }

    group.finish();
}

fn bench_push_slice(c: &mut Criterion) {
    let mut group = c.benchmark_group("RingBuffer/push_slice");

    for size in [256, 512, 1024] {
        let rb: RingBuffer<f32> = RingBuffer::new(8192);
        let input: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let mut drain = vec![0.0f32; size];

        group.bench_with_input(BenchmarkId::new("samples", size), &size, |b, _| {
            b.iter(|| {
                black_box(rb.push_slice(&input));
                rb.pop_slice(&mut drain); // drain to prevent full
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_pop_slice, bench_push_slice);
criterion_main!(benches);
