use criterion::{criterion_group, criterion_main, Criterion};
use step_merger::step::ref_iter::RefIter;

pub fn ref_iter(c: &mut Criterion) {
    c.bench_function("RefIter 2.stp", |b| {
        b.iter(|| {
            let r = RefIter::new("../test_data/2.stp").unwrap();
            r.into_iter().count()
        })
    });
}

criterion_group!(benches, ref_iter);
criterion_main!(benches);
