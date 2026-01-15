//! DSP benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rusty_lights::dsp::{FftProcessor, MelBank, Smoother};

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

        group.bench_with_input(BenchmarkId::new("process", num_bands), &num_bands, |b, _| {
            b.iter(|| {
                black_box(mel_bank.process(black_box(&magnitude)));
            });
        });
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

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("FullPipeline");

    let fft_size = 512;
    let num_bands = 24;
    let num_leds = 300;

    let mut fft = FftProcessor::new(fft_size).unwrap();
    let mut mel_bank = MelBank::new(num_bands, fft_size, 48000, 20.0, 18000.0).unwrap();
    let mut smoother = Smoother::new(num_bands, 0.7);

    // Generate test signal
    let samples: Vec<f32> = (0..fft_size)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
        .collect();

    let mut mel_output = vec![0.0; num_bands];

    group.bench_function("dsp_chain", |b| {
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

criterion_group!(
    benches,
    bench_fft,
    bench_mel_bank,
    bench_smoother,
    bench_full_pipeline,
);

criterion_main!(benches);
