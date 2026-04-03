# Performance Notes

## Ring Buffer Bulk Copy Optimization

The SPSC ring buffer's `push_slice()` and `pop_slice()` were originally implemented
as per-item loops, each calling `push()` / `pop()` individually. This meant N atomic
load-store pairs for N samples — 512 atomic operations per audio frame at the default
FFT size.

The optimized implementation uses `ptr::copy_nonoverlapping` (memcpy) with wrap-around
handling, reducing this to a single atomic load + 1-2 memcpy + single atomic store
regardless of slice size.

### Benchmark results (x86_64, Criterion)

| Operation | Samples | Before | After | Improvement |
|-----------|---------|--------|-------|-------------|
| `pop_slice` | 256 | 661 ns | 24.6 ns | **96.3%** (27x) |
| `pop_slice` | 512 | 1,336 ns | 53.2 ns | **96.0%** (25x) |
| `pop_slice` | 1024 | 2,646 ns | 99.1 ns | **96.3%** (27x) |
| `push_slice` | 256 | 658 ns | 24.7 ns | **96.3%** (27x) |
| `push_slice` | 512 | 1,316 ns | 48.5 ns | **96.3%** (27x) |
| `push_slice` | 1024 | 2,646 ns | 92.2 ns | **96.6%** (29x) |

### Impact on frame budget

At the default 512-sample FFT size on a Raspberry Pi 4 (Cortex-A72 @ 1.5 GHz):

- **Before**: ~1.3 µs per `pop_slice` call (~1% of 150 µs frame budget)
- **After**: ~53 ns per `pop_slice` call (~0.04% of frame budget)

The savings are more pronounced on ARM where atomic operations have higher overhead
relative to memcpy.

### How to reproduce

```bash
cargo bench --bench ringbuffer_bench
```
