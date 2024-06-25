use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Cursor;
use step_merger::step::{STEPReaderLogos, STEPReaderPlain, STEPReaderTrait};

/// Benchmark parsing files with Logos based parser into Vector
pub fn logos_reader_bench(c: &mut Criterion) {
    type Reader<'a> = STEPReaderLogos<Cursor<&'a [u8]>>;

    let s = include_bytes!("../../test_data/1.stp");
    c.bench_function("logos 1.stp", |b| {
        b.iter(|| {
            let it = Reader::new(Cursor::new(s)).unwrap();
            it.collect::<Result<Vec<_>, _>>().unwrap()
        })
    });
    let s = include_bytes!("../../test_data/2.stp");
    c.bench_function("logos 2.stp", |b| {
        b.iter(|| {
            let it = Reader::new(Cursor::new(s)).unwrap();
            it.collect::<Result<Vec<_>, _>>().unwrap()
        })
    });
    let s = include_bytes!("../../test_data/wiki.stp");
    c.bench_function("logos wiki.stp", |b| {
        b.iter(|| {
            let it = Reader::new(Cursor::new(s)).unwrap();
            it.collect::<Result<Vec<_>, _>>().unwrap()
        })
    });
}

/// Benchmark parsing files with plain parser into Vector
pub fn plain_reader_bench(c: &mut Criterion) {
    type Reader<'a> = STEPReaderPlain<Cursor<&'a [u8]>>;

    let s = include_bytes!("../../test_data/1.stp");
    c.bench_function("plain 1.stp", |b| {
        b.iter(|| {
            let it = Reader::new(Cursor::new(s)).unwrap();
            it.collect::<Result<Vec<_>, _>>().unwrap()
        })
    });
    let s = include_bytes!("../../test_data/2.stp");
    c.bench_function("plain 2.stp", |b| {
        b.iter(|| {
            let it = Reader::new(Cursor::new(s)).unwrap();
            it.collect::<Result<Vec<_>, _>>().unwrap()
        })
    });
    let s = include_bytes!("../../test_data/wiki.stp");
    c.bench_function("plain wiki.stp", |b| {
        b.iter(|| {
            let it = Reader::new(Cursor::new(s)).unwrap();
            it.collect::<Result<Vec<_>, _>>().unwrap()
        })
    });
}

criterion_group!(benches, logos_reader_bench, plain_reader_bench,);
criterion_main!(benches);
