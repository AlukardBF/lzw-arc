// #[macro_use]
use criterion::*;
use lzw_arc::lzw;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("archive", |b| {
        b.iter(|| lzw::archive::Compress::new("test", "output", 16).compress())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
