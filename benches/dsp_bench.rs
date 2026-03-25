// SPDX-License-Identifier: GPL-3.0-only
// Copyright (c) 2026 appliedappliance GmbH
//! DSP benchmarks

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rusty_lights::dsp::{BeatDetector, DspConfig, DspPipeline, FftProcessor, MelBank, Smoother};

fn bench_fft(c: &mut Criterion) {
    let mut group = c.benchmark_group("FFT");

    for size in [256, 512, 1024] {
        let mut fft = FftProcessor::new(size).unwrap();

        // Generate test signal
        let samples: Vec<f32> = (0..size)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();

        group.bench_with_input(BenchmarkId::new("process", size), &size, |b, _| {
            b.iter(|| {
                black_box(fft.process(black_box(&samples)));
            });
        });
    }

    group.finish();
}

fn bench_mel_bank(c: &mut Criterion) {
    let mut group = c.benchmark_group("MelBank");

    for num_bands in [16, 24, 32] {
        let mut mel_bank = MelBank::new(num_bands, 512, 48000, 20.0, 18000.0).unwrap();

        // Generate fake magnitude spectrum
        let magnitude: Vec<f32> = (0..257).map(|i| (i as f32 / 257.0)).collect();

        group.bench_with_input(
            BenchmarkId::new("process", num_bands),
            &num_bands,
            |b, _| {
                b.iter(|| {
                    black_box(mel_bank.process(black_box(&magnitude)));
                });
            },
        );
    }

    group.finish();
}

fn bench_smoother(c: &mut Criterion) {
    let mut group = c.benchmark_group("Smoother");

    for size in [24, 64, 128] {
        let mut smoother = Smoother::new(size, 0.7);
        let input: Vec<f32> = (0..size).map(|i| i as f32 / size as f32).collect();
        let mut output = vec![0.0; size];

        group.bench_with_input(BenchmarkId::new("process", size), &size, |b, _| {
            b.iter(|| {
                smoother.process(black_box(&input), black_box(&mut output));
            });
        });
    }

    group.finish();
}

fn bench_beat_detector(c: &mut Criterion) {
    let mut group = c.benchmark_group("BeatDetector");

    let num_bins = 257; // FFT 512 -> 257 bins
    let mut detector = BeatDetector::new(num_bins, 43);

    // Generate fake magnitude spectrum
    let magnitude: Vec<f32> = (0..num_bins)
        .map(|i| (i as f32 / num_bins as f32))
        .collect();

    group.bench_function("process", |b| {
        b.iter(|| {
            black_box(detector.process(black_box(&magnitude)));
        });
    });

    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("FullPipeline");

    let fft_size = 512;
    let num_bands = 24;

    let mut fft = FftProcessor::new(fft_size).unwrap();
    let mut mel_bank = MelBank::new(num_bands, fft_size, 48000, 20.0, 18000.0).unwrap();
    let mut smoother = Smoother::new(num_bands, 0.7);

    // Generate test signal
    let samples: Vec<f32> = (0..fft_size)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
        .collect();

    let mut mel_output = vec![0.0; num_bands];

    group.bench_function("manual_chain", |b| {
        b.iter(|| {
            // FFT
            let magnitude = fft.process(black_box(&samples));

            // Mel Bank
            let mel_bands = mel_bank.process(magnitude);

            // Smoothing
            smoother.process(mel_bands, &mut mel_output);

            black_box(&mel_output);
        });
    });

    group.finish();
}

fn bench_dsp_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("DspPipeline");

    let config = DspConfig::default();
    let mut pipeline = DspPipeline::new(&config).unwrap();

    // Generate test signal
    let samples: Vec<f32> = (0..config.fft_size)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
        .collect();

    group.bench_function("process", |b| {
        b.iter(|| {
            black_box(pipeline.process(black_box(&samples)));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fft,
    bench_mel_bank,
    bench_smoother,
    bench_beat_detector,
    bench_full_pipeline,
    bench_dsp_pipeline,
);

criterion_main!(benches);
