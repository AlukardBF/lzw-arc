// #[macro_use]
use criterion::*;
use lzw_arc::lzw;

fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("compress", |b| {
    //     b.iter(|| lzw::compress("test-file", "output", 16))
    // });
    c.bench_function("compress", |b| {
        b.iter(|| lzw::compress("bench-file", "bench-output", 16))
    });
    // c.bench_function("decompress", |b| {
    //     b.iter(|| lzw::decompress("output", "test-output", 16))
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
