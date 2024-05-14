use chumsky::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};
use logos::Logos;
use std::fs;

// Benchmark parsing files into Vector
pub fn lexer_chumsky_bench(c: &mut Criterion) {
    use step_merger::step::lexer_chumsky::Token;
    let filename = "../test_data/1.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("chumsky 1.stp", |b| b.iter(|| Token::lexer().parse(&s)));
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("chumsky 2.stp", |b| b.iter(|| Token::lexer().parse(&s)));
    let filename = "../test_data/wiki.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("chumsky wiki.stp", |b| b.iter(|| Token::lexer().parse(&s)));
}

pub fn lexer_logos_bench(c: &mut Criterion) {
    use step_merger::step::lexer_logos::Token;
    let filename = "../test_data/1.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("logos 1.stp", |b| {
        b.iter(|| Token::lexer(&s).collect::<Vec<_>>())
    });
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("logos 2.stp", |b| {
        b.iter(|| Token::lexer(&s).collect::<Vec<_>>())
    });
    let filename = "../test_data/wiki.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("logos wiki.stp", |b| {
        b.iter(|| Token::lexer(&s).collect::<Vec<_>>())
    });
}

criterion_group!(benches, lexer_chumsky_bench, lexer_logos_bench,);
criterion_main!(benches);
