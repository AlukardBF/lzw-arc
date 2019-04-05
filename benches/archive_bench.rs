// #[macro_use]
use criterion::*;
use lzw_arc::*;
use bitvec::*;
// use bitvec::{BigEndian, BitVec};
// use lzw_arc;
// use criterion::Criterion;



fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_function("from_u64", |b| {
    //     b.iter(|| {
    //         let mut bits = 10;
    //         lzw::archive::from_u64(73567853241u64, &mut bits);
    //     })
    // });
    // c.bench_function("pop_byte", |b| {
    //     b.iter(|| {
    //         let mut bit_buf: BitVec<BigEndian, u8> = bitvec![BigEndian, u8; 1; 20];
    //         lzw::archive::pop_byte(&mut bit_buf);
    //     })
    // });
    // c.bench_function("from_byte", |b| {
    //     b.iter(|| {
    //         let byte = 243u8;
    //         lzw::archive::from_byte(byte);
    //     })
    // });
    // c.bench_function("add_leading_zeros", |b| {
    //     b.iter(|| {
    //         let bit_buf: BitVec<BigEndian, u8> = bitvec![BigEndian, u8; 1; 20];
    //         lzw::archive::add_leading_zeros(&bit_buf, 32);
    //     })
    // });
    let data = lzw::archive::Data::new("test-pic.jpg", "output-pic");
    c.bench_function("append_to_buf", move |b| {
        b.iter(|| {
            
            let mut bit_buf: BitVec<BigEndian, u8> = BitVec::with_capacity(1024);
            data.append_to_buf(&mut bit_buf, vec![32u8]);
        })
    });
    // c.bench_function("archive", |b| {
    //     b.iter(|| {
    //         lzw::archive::Data::new("test-pic.jpg", "output-pic").archive()
    //     })
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
