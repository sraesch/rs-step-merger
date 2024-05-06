use chumsky::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};
use std::fs;
use step_merger::step::lexer::Token;

// Benchmark parsing files into Vector
pub fn lexer_bench(c: &mut Criterion) {
    let filename = "../test_data/1.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("1.stp", |b| b.iter(|| Token::lexer().parse(&s)));
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("2.stp", |b| b.iter(|| Token::lexer().parse(&s)));
    let filename = "../test_data/wiki.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("wiki.stp", |b| b.iter(|| Token::lexer().parse(&s)));
}

pub fn lexer_bench_iter(c: &mut Criterion) {
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();

    // Benchmark calculating the number of tokens in file 2.stp
    c.bench_function("Count 2.stp", |b| {
        b.iter(|| Token::lexer_iter().count().parse(&s))
    });
    // Benchmark calculating the minimum, maximum and number of references
    // c.bench_function("2.stp", |b| b.iter(|| Token::lexer_iter().));
}

criterion_group!(benches, lexer_bench, lexer_bench_iter);
criterion_main!(benches);
